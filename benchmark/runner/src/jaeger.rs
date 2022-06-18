pub mod api {
    tonic::include_proto!("jaeger.api_v2");
}

use crate::jaeger::api::{FindTracesRequest, TraceQueryParameters};
use crate::model::{InvocationStats, Mode};
use anyhow::{anyhow, Context};
use api::query_service_client;
use std::collections::HashMap;
use std::num::TryFromIntError;
use std::time::SystemTime;

struct Query<'a> {
    service_name: &'a str,
    root_operation_name: &'a str,
    processing_operation_name: &'a str,
}

pub async fn find_durations(
    endpoint: &str,
    mode: Mode,
    workflow_name: &str,
) -> anyhow::Result<Vec<InvocationStats>> {
    let query: Query = match mode {
        Mode::WasmLocal => Query {
            service_name: "wasm-workflows-plugin",
            root_operation_name: "request.execute_template",
            processing_operation_name: "wasm.execute_mod",
        },
        Mode::WasmDistributed => Query {
            service_name: "wasm-workflows-plugin",
            root_operation_name: "request.execute_template",
            processing_operation_name: "container.running",
        },
        Mode::Container => Query {
            service_name: "image-processor",
            root_operation_name: "processor.run",
            processing_operation_name: "processor.run",
        },
    };
    let mut client = query_service_client::QueryServiceClient::connect(
        tonic::transport::Endpoint::from_shared(endpoint.to_string())?,
    )
    .await?;
    let jaeger_query = TraceQueryParameters {
        service_name: query.service_name.into(),
        operation_name: query.root_operation_name.into(),
        tags: HashMap::from([("workflow_name".to_string(), workflow_name.to_owned())]),
        start_time_min: None,
        start_time_max: None,
        duration_min: None,
        duration_max: None,
        search_depth: 256,
    };
    tracing::trace!(?jaeger_query, "Jaeger Query");
    let response = client
        .find_traces(FindTracesRequest {
            query: Some(jaeger_query),
        })
        .await?;
    let mut traces = response.into_inner();
    let mut results: Vec<InvocationStats> = Vec::new();
    while let Some(trace) = traces.message().await? {
        tracing::trace!(?trace, "Got trace");
        for span in trace.spans {
            if span.operation_name == query.processing_operation_name {
                if let Some(duration) = span.duration {
                    if let Some(start_time) = span.start_time {
                        let started_at =
                            SystemTime::try_from(start_time).context("Calculting start time")?;
                        let duration: usize = (duration.seconds * 1000
                            + (duration.nanos / 1000 / 1000) as i64)
                            .try_into()
                            .map_err(|why: TryFromIntError| {
                                anyhow!(why).context("Calculating span duration as ms")
                            })?;
                        results.push(InvocationStats {
                            timestamp: started_at.into(),
                            processing_ms: duration,
                        });
                    } else {
                        tracing::warn!("Span skipped because start time is missing")
                    }
                } else {
                    tracing::warn!("Span skipped because duration is missing")
                }
            }
        }
    }
    Ok(results)
}
