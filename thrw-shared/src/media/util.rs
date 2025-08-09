use std::{path::PathBuf, process::Command};

use super::prelude::*;
use ffmpeg_sidecar::command::FfmpegCommand;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
// use ffmpeg_next::format::context::Input;
use sqlx::{Pool, Postgres};

// pub async fn get_media_context(
// 	path: PathBuf,
// ) -> Result<Input, MediaError> {
// 	// ffmpeg_next::format::input(path)
// }

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaCodecType {
	Audio,
	Video,
}
impl ToString for MediaCodecType {
	fn to_string(&self) -> String {
		match self {
			MediaCodecType::Audio => "audio",
			MediaCodecType::Video => "video",
		}.to_string()
	}
}

#[derive(Debug, Clone, Deserialize)]
pub struct FFProbeMediaOutput {
    pub streams: Vec<Stream>,
    pub format: Format,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize)]
pub struct Stream {
    pub index: i32,
    pub codec_name: Option<String>,
    pub codec_long_name: Option<String>,
    pub profile: Option<String>,
    pub codec_type: MediaCodecType,
    pub codec_tag_string: Option<String>,
    pub codec_tag: Option<String>,
    
    // Video-specific
    pub width: Option<i16>,
    pub height: Option<i16>,
    pub coded_width: Option<i16>,
    pub coded_height: Option<i16>,
    pub closed_captions: Option<i32>,
    pub film_grain: Option<i32>,
    pub has_b_frames: Option<i32>,
    pub sample_aspect_ratio: Option<String>,
    pub display_aspect_ratio: Option<String>,
    pub pix_fmt: Option<String>,
    pub level: Option<i32>,
    pub color_range: Option<String>,
    pub color_space: Option<String>,
    pub color_transfer: Option<String>,
    pub color_primaries: Option<String>,
    pub r#ref: Option<i32>,

    // Audio-specific
    pub sample_fmt: Option<String>,
	#[serde_as(as = "Option<DisplayFromStr>")]
    pub sample_rate: Option<i32>,
    pub channels: Option<i32>,
    pub channel_layout: Option<String>,
    pub bits_per_sample: Option<i32>,
    pub initial_padding: Option<i32>,
    pub extradata_size: Option<i32>,

    // Timing
    pub r_frame_rate: Option<String>,
    pub avg_frame_rate: Option<String>,
    pub time_base: Option<String>,
    pub start_pts: Option<i64>,
    pub start_time: Option<String>,

    pub disposition: Option<Disposition>,
    pub tags: Option<StreamTags>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Disposition {
    pub default: u8,
    pub dub: u8,
    pub original: u8,
    pub comment: u8,
    pub lyrics: u8,
    pub karaoke: u8,
    pub forced: u8,
    pub hearing_impaired: u8,
    pub visual_impaired: u8,
    pub clean_effects: u8,
    pub attached_pic: u8,
    pub timed_thumbnails: u8,
    pub non_diegetic: u8,
    pub captions: u8,
    pub descriptions: u8,
    pub metadata: u8,
    pub dependent: u8,
    pub still_image: u8,
    pub multilayer: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StreamTags {
    #[serde(rename = "HANDLER_NAME")]
    pub handler_name: Option<String>,
    #[serde(rename = "VENDOR_ID")]
    pub vendor_id: Option<String>,
    #[serde(rename = "DURATION")]
    pub duration: Option<String>,
    pub language: Option<String>,
}

#[serde_as]
#[derive(Debug, Clone, Deserialize)]
pub struct Format {
    pub filename: String,
    pub nb_streams: i32,
    pub nb_programs: i32,
    pub nb_stream_groups: Option<i32>,
    pub format_name: String,
    pub format_long_name: String,
    pub start_time: Option<String>,
	#[serde_as(as = "Option<DisplayFromStr>")]
    pub duration: Option<f64>,
	#[serde_as(as = "DisplayFromStr")]
    pub size: u64,
	#[serde_as(as = "Option<DisplayFromStr>")]
    pub bit_rate: Option<i32>,
    pub probe_score: Option<i32>,
    pub tags: Option<FormatTags>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FormatTags {
    #[serde(rename = "COMPATIBLE_BRANDS")]
    pub compatible_brands: Option<String>,
    #[serde(rename = "MAJOR_BRAND")]
    pub major_brand: Option<String>,
    #[serde(rename = "MINOR_VERSION")]
    pub minor_version: Option<String>,
    #[serde(rename = "ENCODER")]
    pub encoder: Option<String>,
}

fn get_file_metadata_output(
	path: PathBuf,
) -> Result<std::process::Output, MediaError> {
	let path = path.to_string_lossy();
	Command::new("ffprobe")
		.args([
			"-v", "error",
			"-print_format", "json",
			"-show_format",
			"-show_streams",
			&path
		])
		.output()
		.map_err(MediaError::Io)
}

pub async fn get_media_file_metadata(
	path: PathBuf,
) -> Result<FFProbeMediaOutput, MediaError> {
	let output  = get_file_metadata_output(path)?;
	
	serde_json::from_slice(&output.stdout).map_err(MediaError::Json)
}

pub async fn init_media(
	db_pool: &Pool<Postgres>,
) -> Result<(), MediaError> {
	println!("initializing media...");
	ffmpeg_sidecar::download::auto_download()
		.map_err(MediaError::Ffmpeg)?
	;

	#[cfg(debug_assertions)]
	{
		let _ = crate::util::copy_dir_all(
			std::path::PathBuf::from("./site"),
			std::path::PathBuf::from("./target/site")
		);
	}
	

	Ok(())
}