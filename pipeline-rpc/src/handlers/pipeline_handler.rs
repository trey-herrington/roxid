use crate::error::RpcResult;
use pipeline_service::pipeline::{
    ExecutionContext, ExecutionEvent, Pipeline, PipelineExecutor, PipelineParser,
};
use std::path::Path;
use tokio::sync::mpsc;

#[derive(Debug)]
pub struct PipelineHandler;

impl PipelineHandler {
    pub fn new() -> Self {
        Self
    }

    pub fn parse_from_file<P: AsRef<Path>>(&self, path: P) -> RpcResult<Pipeline> {
        Ok(PipelineParser::from_file(path)?)
    }

    pub fn parse_from_str(&self, content: &str) -> RpcResult<Pipeline> {
        Ok(PipelineParser::from_str(content)?)
    }

    pub async fn execute_pipeline(
        &self,
        pipeline: Pipeline,
        working_dir: String,
        event_sender: Option<mpsc::UnboundedSender<ExecutionEvent>>,
    ) -> RpcResult<()> {
        let context = ExecutionContext::new(pipeline.name.clone(), working_dir);
        let executor = PipelineExecutor::new(context);
        executor.execute(pipeline, event_sender).await;
        Ok(())
    }

    pub fn create_event_channel() -> (
        mpsc::UnboundedSender<ExecutionEvent>,
        mpsc::UnboundedReceiver<ExecutionEvent>,
    ) {
        mpsc::unbounded_channel()
    }
}

impl Default for PipelineHandler {
    fn default() -> Self {
        Self::new()
    }
}
