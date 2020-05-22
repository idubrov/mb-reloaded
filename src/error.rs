use sdl2::render::TargetRenderError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error("SDL error: {0}")]
    SdlError(String),

    #[error("Target render error")]
    TargetRenderError(#[from] TargetRenderError),
}
