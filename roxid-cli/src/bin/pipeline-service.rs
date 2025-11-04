use pipeline_service::grpc::proto::{
    pipeline_service_server::{PipelineService, PipelineServiceServer},
    ExecutePipelineRequest, ExecutionEvent, ParsePipelineRequest, ParsePipelineResponse,
};
use pipeline_service::pipeline::{ExecutionContext, PipelineExecutor, PipelineParser};
use tonic::{transport::Server, Request, Response, Status};
use tokio_stream::wrappers::UnboundedReceiverStream;

#[derive(Debug, Default)]
pub struct PipelineServiceImpl;

#[tonic::async_trait]
impl PipelineService for PipelineServiceImpl {
    async fn parse_pipeline(
        &self,
        request: Request<ParsePipelineRequest>,
    ) -> Result<Response<ParsePipelineResponse>, Status> {
        let req = request.into_inner();

        let pipeline = match req.source {
            Some(pipeline_service::grpc::proto::parse_pipeline_request::Source::FilePath(
                path,
            )) => PipelineParser::from_file(&path).map_err(|e| {
                Status::invalid_argument(format!("Failed to parse pipeline from file: {}", e))
            })?,
            Some(pipeline_service::grpc::proto::parse_pipeline_request::Source::Content(
                content,
            )) => PipelineParser::parse(&content).map_err(|e| {
                Status::invalid_argument(format!("Failed to parse pipeline from string: {}", e))
            })?,
            None => {
                return Err(Status::invalid_argument(
                    "Must provide either file_path or content",
                ))
            }
        };

        Ok(Response::new(ParsePipelineResponse {
            pipeline: Some(pipeline.into()),
        }))
    }

    type ExecutePipelineStream = UnboundedReceiverStream<Result<ExecutionEvent, Status>>;

    async fn execute_pipeline(
        &self,
        request: Request<ExecutePipelineRequest>,
    ) -> Result<Response<Self::ExecutePipelineStream>, Status> {
        let req = request.into_inner();

        let pipeline = req
            .pipeline
            .ok_or_else(|| Status::invalid_argument("Pipeline is required"))?
            .into();

        let context = ExecutionContext::new(
            req.working_dir.clone(),
            req.working_dir,
        );

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let (grpc_tx, grpc_rx) = tokio::sync::mpsc::unbounded_channel();

        // Spawn the executor
        tokio::spawn(async move {
            let executor = PipelineExecutor::new(context);
            executor.execute(pipeline, Some(tx)).await;
        });

        // Forward events from executor to gRPC stream
        tokio::spawn(async move {
            let mut rx = rx;
            while let Some(event) = rx.recv().await {
                let proto_event: pipeline_service::grpc::proto::ExecutionEvent = event.into();
                if grpc_tx.send(Ok(proto_event)).is_err() {
                    break;
                }
            }
        });

        Ok(Response::new(UnboundedReceiverStream::new(grpc_rx)))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let service = PipelineServiceImpl::default();

    println!("Pipeline gRPC server listening on {}", addr);

    Server::builder()
        .add_service(PipelineServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
