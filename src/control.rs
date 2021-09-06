use std::io::{Stdout, Write};

use crate::cam_info::CamInfo;
use anyhow::*;
use futures::{FutureExt, Stream, StreamExt};
use gstreamer::prelude::ObjectExt;
use gstreamer_video::VideoOrientationMethod;
use log::*;
use termion::{event::Key, input::TermRead};
use tokio::{signal::unix::SignalKind, sync::oneshot::Sender};

enum Command {
    Quit,
    ZoomIn,
    ZoomOut,
    Nothing,
    PanLeft,
    PanRight,
    PanUp,
    PanDown,
    QualityUp,
    QualityDown,
    Flip,
}

#[derive(Debug)]
struct CamControl {
    quit: Sender<()>,
    port: u16,
    cam_info: CamInfo,
    stdout: Stdout,
    video_flip: gstreamer::Element,
    flip_method: VideoOrientationMethod,
}

impl CamControl {
    async fn new(
        quit: Sender<()>,
        port: u16,
        video_flip: gstreamer::Element,
    ) -> Result<CamControl, (Error, Sender<()>)> {
        match (get_cam_info(port, true).await, get_flip_method(&video_flip)) {
            (Ok(cam_info), Ok(flip_method)) => Ok(CamControl {
                quit,
                port,
                cam_info,
                stdout: std::io::stdout(),
                video_flip,
                flip_method,
            }),
            (Err(e), _) => Err((e, quit)),
            (_, Err(e)) => Err((e, quit)),
        }
    }

    async fn refresh(&mut self) -> Result<()> {
        let new = get_cam_info(self.port, false).await?;

        self.cam_info.curvals = new.curvals;

        self.flip_method = get_flip_method(&self.video_flip)?;

        Ok(())
    }

    fn display_status(&mut self) -> Result<()> {
        if log_enabled!(log::Level::Error) {
            if let Some((zoom_idx, zoom_end)) = self.zoom_index() {
                let p = (100 * zoom_idx) / zoom_end;
                let q = self.cam_info.curvals.quality;

                let f = match self.flip_method {
                    VideoOrientationMethod::Horiz => ", Flip: H",
                    VideoOrientationMethod::Vert => ", Flip: V",
                    _ => "         ",
                };

                write!(self.stdout, "Zoom: {:2} %, Quality: {:2} %{}\r", p, q, f)?;
                self.stdout.flush()?;
            }
        }

        Ok(())
    }

    fn zoom_index(&self) -> Option<(usize, usize)> {
        let h = &self.cam_info.avail.as_ref()?.zoom;
        let idx = h.iter().position(|e| {
            if let Ok(i) = e.parse::<u16>() {
                i == self.cam_info.curvals.zoom
            } else {
                false
            }
        })?;

        Some((idx, h.len()))
    }

    fn increment_zoom_index(&self) -> Result<usize> {
        let (idx, len) = self
            .zoom_index()
            .ok_or_else(|| anyhow!("internal error while zooming in"))?;

        if idx < len {
            Ok(idx + 1)
        } else {
            Ok(idx)
        }
    }

    fn decrement_zoom_index(&self) -> Result<usize> {
        let (idx, _) = self
            .zoom_index()
            .ok_or_else(|| anyhow!("internal error while zooming out"))?;

        if idx > 0 {
            Ok(idx - 1)
        } else {
            Ok(0)
        }
    }
}

fn get_flip_method(video_flip: &gstreamer::Element) -> Result<VideoOrientationMethod> {
    let m: VideoOrientationMethod = video_flip.property("video-direction")?.get()?;

    Ok(m)
}

pub async fn get_cam_info(port: u16, init: bool) -> Result<CamInfo> {
    let show = if init { "1" } else { "0" };
    let c = reqwest::get(format!(
        "http://127.0.0.1:{}/status.json?show_avail={}",
        port, show
    ))
    .await?
    .json::<CamInfo>()
    .await?;

    trace!("{:?}", c);

    Ok(c)
}

pub async fn process_commands(port: u16, video_flip: gstreamer::Element) -> Result<()> {
    let (tx, rx) = tokio::sync::oneshot::channel();

    let j = tokio::task::spawn(async move {
        match CamControl::new(tx, port, video_flip).await {
            Ok(c) => {
                match process_commands_inner(c).await {
                    Ok(_) => {}
                    Err(e) => error!("{}", e),
                };
            }
            Err((e, tx)) => {
                debug!("{}", e);
                warn!("failed to connect to droidcam controls; disabling device control.");
                match process_commands_fallback(tx).await {
                    Ok(_) => {}
                    Err(e) => error!("{}", e),
                };
            }
        };
    });

    futures::future::select(rx, j).await;

    Ok(())
}

async fn process_commands_inner(control: CamControl) -> Result<()> {
    let mut cmds = input_commands().boxed();
    let mut control = control;

    writeln!(
        control.stdout,
        "Press 'q': quit, 'z'/'Z': zoom, 't'/'T': quality, 'f': flip, arrows: pan.\r"
    )?;
    control.display_status()?;
    while let Some(cmd) = cmds.next().await {
        match cmd {
            Command::Quit => {
                if log_enabled!(log::Level::Error) {
                    write!(control.stdout, "{}", termion::clear::CurrentLine)?;
                    control.stdout.flush()?;
                }
                control
                    .quit
                    .send(())
                    .map_err(|_| anyhow!("broken channel"))?;
                break;
            }
            Command::Nothing => {}
            Command::ZoomIn => {
                let new_zoom = &control.increment_zoom_index()?;
                reqwest::get(format!(
                    "http://127.0.0.1:{}/ptz?zoom={}",
                    control.port, new_zoom
                ))
                .await?;
            }
            Command::ZoomOut => {
                let new_zoom = &control.decrement_zoom_index()?;
                reqwest::get(format!(
                    "http://127.0.0.1:{}/ptz?zoom={}",
                    control.port, new_zoom
                ))
                .await?;
            }
            Command::PanLeft => {
                let new_x = &control.cam_info.curvals.crop_x.max(1) - 1;
                reqwest::get(format!(
                    "http://127.0.0.1:{}/settings/crop_x?set={}",
                    control.port, new_x
                ))
                .await?;
            }
            Command::PanRight => {
                let new_x = &control.cam_info.curvals.crop_x + 1;
                reqwest::get(format!(
                    "http://127.0.0.1:{}/settings/crop_x?set={}",
                    control.port, new_x
                ))
                .await?;
            }
            Command::PanUp => {
                let new_x = &control.cam_info.curvals.crop_y.max(1) - 1;
                reqwest::get(format!(
                    "http://127.0.0.1:{}/settings/crop_y?set={}",
                    control.port, new_x
                ))
                .await?;
            }
            Command::PanDown => {
                let new_x = &control.cam_info.curvals.crop_y + 1;
                reqwest::get(format!(
                    "http://127.0.0.1:{}/settings/crop_y?set={}",
                    control.port, new_x
                ))
                .await?;
            }
            Command::QualityUp => {
                let new_q = &control.cam_info.curvals.quality + 1;
                reqwest::get(format!(
                    "http://127.0.0.1:{}/settings/quality?set={}",
                    control.port, new_q
                ))
                .await?;
            }
            Command::QualityDown => {
                let new_q = &control.cam_info.curvals.quality - 1;
                reqwest::get(format!(
                    "http://127.0.0.1:{}/settings/quality?set={}",
                    control.port, new_q
                ))
                .await?;
            }
            Command::Flip => {
                use VideoOrientationMethod::*;
                let new = match control.flip_method {
                    Identity | _90r | _180 | _90l => Horiz,
                    Horiz => Vert,
                    _ => Identity,
                };
                control.video_flip.set_property("video-direction", new)?;
            }
        }

        control.refresh().await?;
        control.display_status()?;
    }

    Ok(())
}

async fn process_commands_fallback(quit: Sender<()>) -> Result<()> {
    let mut cmds = input_commands().boxed();
    let mut stdout = std::io::stdout();

    writeln!(stdout, "Press 'q' to quit.\r")?;
    while let Some(cmd) = cmds.next().await {
        if let Command::Quit = cmd {
            if log_enabled!(log::Level::Error) {
                write!(stdout, "{}", termion::clear::CurrentLine)?;
                stdout.flush()?;
            }
            quit.send(()).map_err(|_| anyhow!("broken channel"))?;
            break;
        }
    }

    Ok(())
}

pub async fn stop_signals() -> Result<()> {
    let mut i = tokio::signal::unix::signal(SignalKind::interrupt())?;
    let mut t = tokio::signal::unix::signal(SignalKind::terminate())?;
    let mut q = tokio::signal::unix::signal(SignalKind::quit())?;

    let is = i.recv().boxed_local();
    let ts = t.recv().boxed_local();
    let qs = q.recv().boxed_local();

    futures::future::select_all(vec![is, ts, qs]).await;

    Ok(())
}

fn input_commands() -> impl Stream<Item = Command> {
    let keys = stdin_stream();

    use Command::*;
    keys.map(|k| match k {
        Key::Char('q') => Quit,
        Key::Char('z') => ZoomIn,
        Key::Char('Z') => ZoomOut,
        Key::Char('t') => QualityUp,
        Key::Char('T') => QualityDown,
        Key::Char('f') => Flip,
        Key::Left => PanLeft,
        Key::Right => PanRight,
        Key::Up => PanUp,
        Key::Down => PanDown,
        _ => Nothing,
    })
}

fn stdin_stream() -> impl Stream<Item = Key> {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Key>(16);

    tokio::task::spawn_blocking(move || {
        let mut keys = std::io::stdin().keys();
        while let Some(Ok(k)) = keys.next() {
            tx.blocking_send(k).unwrap();
        }
    });

    let stream = async_stream::stream! {
        while let Some(item) = rx.recv().await {
            yield item;
        }
    };

    stream
}
