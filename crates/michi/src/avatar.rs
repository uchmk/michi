//! Author avatars in the graph (issue #84). A commit whose author's e-mail is
//! a GitHub no-reply address gets that account's picture; everyone else, and
//! everyone while the picture is coming or cannot come, gets a circle with
//! their initials. The pictures are fetched with the system's `curl` (as ito's
//! update check does) on one thread of their own, decoded there, and kept for
//! the session. The UI thread never waits for any of it. `[view] avatars =
//! false` in config.toml (or the Settings switch) stops every request.

use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};

use egui::epaint::{Mesh, Vertex};
use egui::{pos2, Align2, Color32, Context, FontId, Pos2, Shape, TextureHandle, TextureOptions};
use michi_core::avatar::{hue, initials, GhUser};

/// The side of the picture asked for, in pixels (drawn at 16 points, so 3x).
const PX: u32 = 48;
/// Pictures bigger than this are not read.
const MAX_BYTES: &str = "2000000";

enum Slot {
    /// Asked for, not here yet.
    Loading,
    Ready(TextureHandle),
    /// Could not be had (offline, no such account, not a picture): the circle stays.
    Missing,
}

struct Fetched {
    key: String,
    picture: Option<TextureHandle>,
}

#[derive(Default)]
pub struct Avatars {
    slots: HashMap<String, Slot>,
    /// Requests to the worker (started with the first one) and its answers.
    ask: Option<Sender<(String, String)>>,
    got: Option<Receiver<Fetched>>,
}

impl Avatars {
    /// Take in what the worker has finished.
    fn pump(&mut self) {
        let Some(rx) = &self.got else { return };
        while let Ok(f) = rx.try_recv() {
            self.slots.insert(f.key, f.picture.map_or(Slot::Missing, Slot::Ready));
        }
    }

    /// The picture for the author with `email`, asking for it the first time;
    /// `None` while it is coming, when it cannot come, and when the address
    /// is not a GitHub account's. Never blocks.
    pub fn picture(&mut self, ctx: &Context, email: &str) -> Option<egui::TextureId> {
        self.pump();
        let user = GhUser::of_email(email)?;
        let key = user.key();
        match self.slots.get(&key) {
            Some(Slot::Ready(t)) => return Some(t.id()),
            Some(_) => return None,
            None => {}
        }
        if self.ask.is_none() {
            let (ask_tx, ask_rx) = channel::<(String, String)>();
            let (got_tx, got_rx) = channel();
            let ctx = ctx.clone();
            let spawned = std::thread::Builder::new().name("avatars".into()).spawn(move || {
                while let Ok((key, url)) = ask_rx.recv() {
                    let picture = fetch(&url).map(|img| ctx.load_texture(format!("avatar-{key}"), img, TextureOptions::LINEAR));
                    if got_tx.send(Fetched { key, picture }).is_err() {
                        break;
                    }
                    ctx.request_repaint();
                }
            });
            if spawned.is_err() {
                self.slots.insert(key, Slot::Missing);
                return None;
            }
            (self.ask, self.got) = (Some(ask_tx), Some(got_rx));
        }
        let sent = self.ask.as_ref().is_some_and(|t| t.send((key.clone(), user.url(PX))).is_ok());
        self.slots.insert(key, if sent { Slot::Loading } else { Slot::Missing });
        None
    }
}

/// The picture at `url`, decoded and cut down to the size drawn. Blocks:
/// only the worker calls it.
fn fetch(url: &str) -> Option<egui::ColorImage> {
    let mut cmd = std::process::Command::new("curl");
    cmd.args(["-sfL", "--max-time", "10", "--max-filesize", MAX_BYTES, "-A", "michi", url]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW: no console flashes up from a window app.
        cmd.creation_flags(0x0800_0000);
    }
    let out = cmd.stdin(std::process::Stdio::null()).stderr(std::process::Stdio::null()).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let img = image::load_from_memory(&out.stdout).ok()?;
    let img = img.resize_to_fill(PX, PX, image::imageops::FilterType::Triangle).to_rgba8();
    Some(egui::ColorImage::from_rgba_unmultiplied([img.width() as usize, img.height() as usize], img.as_raw()))
}

/// A circle of diameter `d` centred on `at`: the picture when there is one,
/// else the initials of `name` on a colour that is the author's own.
pub fn draw(p: &egui::Painter, at: Pos2, d: f32, picture: Option<egui::TextureId>, name: &str) {
    let r = d / 2.0;
    match picture {
        Some(tex) => {
            p.add(Shape::mesh(disc(at, r, tex)));
        }
        None => {
            let fill = egui::ecolor::Hsva::new(hue(name), 0.45, 0.55, 1.0);
            p.circle_filled(at, r, Color32::from(fill));
            p.text(at, Align2::CENTER_CENTER, initials(name), FontId::proportional(d * 0.55), Color32::WHITE);
        }
    }
}

/// A disc as a fan of triangles with the whole picture mapped over its square.
fn disc(at: Pos2, r: f32, tex: egui::TextureId) -> Mesh {
    const N: u32 = 24;
    let mut m = Mesh::with_texture(tex);
    let vertex =
        |pos: Pos2| Vertex { pos, uv: pos2((pos.x - at.x) / (2.0 * r) + 0.5, (pos.y - at.y) / (2.0 * r) + 0.5), color: Color32::WHITE };
    m.vertices.push(vertex(at));
    for i in 0..N {
        let a = i as f32 / N as f32 * std::f32::consts::TAU;
        m.vertices.push(vertex(pos2(at.x + r * a.cos(), at.y + r * a.sin())));
    }
    for i in 0..N {
        m.indices.extend([0, 1 + i, 1 + (i + 1) % N]);
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_disc_has_a_fan_of_triangles_with_uvs_inside_the_picture() {
        let m = disc(pos2(10.0, 10.0), 8.0, egui::TextureId::default());
        assert_eq!(m.vertices.len(), 25);
        assert_eq!(m.indices.len(), 72);
        assert!(m.vertices.iter().all(|v| (0.0..=1.0).contains(&v.uv.x) && (0.0..=1.0).contains(&v.uv.y)));
        assert!(m.indices.iter().all(|&i| (i as usize) < m.vertices.len()));
    }

    #[test]
    fn an_address_that_is_not_github_is_never_asked_for() {
        let ctx = Context::default();
        let mut a = Avatars::default();
        assert!(a.picture(&ctx, "ann@example.com").is_none());
        assert!(a.ask.is_none(), "no worker, no request");
    }
}
