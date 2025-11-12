use candle::{Device, Result, safetensors};
use candle_transformers::models::whisper::{self as whisper, Config, Whisper};
use hf_hub::api::sync::Api;
use hf_hub::{Repo, RepoType};
use std::fs::File;
use std::path::Path;

pub fn load_model(
    model_path: &str,
    tokenizer_path: &str,
    config_path: &str,
    quantized: bool,
) -> Result<(Whisper, whisper::tokenizer::Tokenizer)> {
    let device = Device::Cpu;

    let config: Config = serde_json::from_reader(File::open(config_path).map_err(|e| candle::Error::Msg(e.to_string()))?).map_err(|e| candle::Error::Msg(e.to_string()))?;
    let tokenizer = whisper::tokenizer::Tokenizer::from_file(tokenizer_path).map_err(|e| candle::Error::Msg(e.to_string()))?;

    let mut vb = candle_nn::VarBuilder::from_safetensors(vec![model_path.to_string()], candle::DType::F32, &device)?;
    let model = Whisper::load(&vb, config)?;
    Ok((model, tokenizer))
}
