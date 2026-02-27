use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use vvtv_types::{AssetItem, QueueEntry};

pub struct HlsStreamer;

pub struct HlsOutput {
    pub playlist_path: PathBuf,
    pub segment_count_estimate: usize,
}

impl HlsStreamer {
    #[must_use]
    pub fn render_playlist(queue: &[QueueEntry]) -> String {
        let mut out = String::from("#EXTM3U\n#EXT-X-VERSION:3\n");
        for entry in queue {
            out.push_str("#EXTINF:600,\n");
            out.push_str(&format!("assets/{}.ts\n", entry.asset_id));
        }
        out
    }

    pub fn build_hls(
        queue: &[QueueEntry],
        assets: &[AssetItem],
        output_dir: impl AsRef<Path>,
    ) -> Result<HlsOutput> {
        let output_dir = output_dir.as_ref();
        fs::create_dir_all(output_dir)
            .with_context(|| format!("failed to create hls output dir {}", output_dir.display()))?;

        if !command_exists("ffmpeg") {
            let playlist = Self::render_playlist(queue);
            let playlist_path = output_dir.join("index.m3u8");
            fs::write(&playlist_path, playlist).with_context(|| {
                format!(
                    "failed writing fallback playlist at {}",
                    playlist_path.display()
                )
            })?;
            return Ok(HlsOutput {
                playlist_path,
                segment_count_estimate: queue.len(),
            });
        }

        let asset_by_id: HashMap<&str, &AssetItem> = assets
            .iter()
            .map(|asset| (asset.asset_id.as_str(), asset))
            .collect();
        let concat_path = output_dir.join("concat.txt");
        let mut concat_manifest = String::new();

        for entry in queue {
            if let Some(asset) = asset_by_id.get(entry.asset_id.as_str()) {
                let abs_path = fs::canonicalize(&asset.local_path)
                    .unwrap_or_else(|_| PathBuf::from(&asset.local_path));
                concat_manifest.push_str(&format!(
                    "file '{}'\n",
                    escape_path_for_concat(abs_path.to_string_lossy().as_ref())
                ));
            }
        }

        if concat_manifest.is_empty() {
            anyhow::bail!("no assets available to generate HLS");
        }

        fs::write(&concat_path, concat_manifest).with_context(|| {
            format!(
                "failed writing concat manifest at {}",
                concat_path.display()
            )
        })?;

        let playlist_path = output_dir.join("index.m3u8");
        let segment_pattern = output_dir.join("segment_%05d.ts");

        let status = Command::new("ffmpeg")
            .args([
                "-loglevel",
                "error",
                "-nostats",
                "-y",
                "-f",
                "concat",
                "-safe",
                "0",
                "-i",
                concat_path.to_str().unwrap_or_default(),
                "-c",
                "copy",
                "-f",
                "hls",
                "-hls_time",
                "6",
                "-hls_list_size",
                "0",
                "-hls_flags",
                "independent_segments",
                "-hls_segment_filename",
                segment_pattern.to_str().unwrap_or_default(),
                playlist_path.to_str().unwrap_or_default(),
            ])
            .status()
            .context("failed to start ffmpeg for hls generation")?;

        if !status.success() {
            anyhow::bail!("ffmpeg hls generation failed");
        }

        Ok(HlsOutput {
            playlist_path,
            segment_count_estimate: queue.len(),
        })
    }
}

fn command_exists(cmd: &str) -> bool {
    Command::new(cmd).arg("-version").output().is_ok()
}

fn escape_path_for_concat(path: &str) -> String {
    path.replace('\\', "\\\\").replace('\'', "'\\''")
}
