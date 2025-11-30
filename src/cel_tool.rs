use cel::{Context, Program, Value as CelValueEnum};
use rmcp::{
    ErrorData, ServerHandler,
    handler::server::{tool::ToolRouter, wrapper::Json, wrapper::Parameters},
    tool, tool_handler, tool_router,
};
use rmcp::{
    model::{Implementation, ServerCapabilities, ServerInfo},
    schemars::{self, JsonSchema},
    serde::{Deserialize, Serialize},
    serde_json::{self, Map, Value},
};
use std::convert::From;
use tokio::sync::{mpsc, oneshot};

/// A newtype wrapper to implement `From<CelJsonValue> for Value`
struct CelJsonValue(CelValueEnum);

/// Converts a `cel::Value` into a `serde_json::Value`.
impl From<CelJsonValue> for Value {
    fn from(wrapper: CelJsonValue) -> Self {
        match wrapper.0 {
            CelValueEnum::Null => Value::Null,
            CelValueEnum::Bool(b) => Value::Bool(b),
            CelValueEnum::Int(i) => serde_json::json!(i),
            CelValueEnum::UInt(u) => serde_json::json!(u),
            CelValueEnum::Float(f) => serde_json::json!(f),
            CelValueEnum::String(s) => Value::String(s.to_string()),
            CelValueEnum::Bytes(b) => Value::String(String::from_utf8_lossy(&b).to_string()),
            CelValueEnum::List(list) => {
                let values: &[CelValueEnum] = &list;
                Value::Array(
                    values
                        .iter()
                        .map(|v| CelJsonValue(v.clone()).into())
                        .collect(),
                )
            }
            CelValueEnum::Map(map_obj) => {
                let mut json_map = Map::new();
                for (key, val) in map_obj.map.iter() {
                    let cel_key_value: CelValueEnum = key.into();
                    let key_str = match cel_key_value {
                        CelValueEnum::String(s) => s.to_string(),
                        _ => format!("{:?}", cel_key_value),
                    };
                    json_map.insert(key_str, CelJsonValue(val.clone()).into());
                }
                Value::Object(json_map)
            }
            // For other CEL types (like Type), just return a string representation.
            cel_value => Value::String(format!("{:?}", cel_value)),
        }
    }
}

/// Compiles and executes a CEL expression with a given context.
fn real_evaluate(expression: &str, context: &Value) -> Result<Value, String> {
    let mut ctx = Context::default();
    if let Value::Object(map) = context {
        for (key, value) in map {
            ctx.add_variable(key, value.clone())
                .map_err(|e| format!("Context error: {}", e))?;
        }
    }

    let prog = Program::compile(expression).map_err(|e| format!("CEL compile error: {}", e))?;

    let result = prog
        .execute(&ctx)
        .map_err(|e| format!("CEL execution error: {}", e))?;

    Ok(CelJsonValue(result).into())
}

pub type EvalResponse = Result<Value, String>;

#[derive(Debug)]
pub struct EvalRequest {
    pub expression: String,
    pub context: Value,
    pub responder: oneshot::Sender<EvalResponse>,
}

pub async fn evaluator_service(mut receiver: mpsc::Receiver<EvalRequest>) {
    while let Some(request) = receiver.recv().await {
        let response = real_evaluate(&request.expression, &request.context);
        if request.responder.send(response).is_err() {
            eprintln!("Failed to send evaluation response");
        }
    }
}

#[derive(Clone)]
pub struct CelTool {
    pub eval_tx: mpsc::Sender<EvalRequest>,
    tool_router: ToolRouter<Self>,
}

#[derive(Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
#[serde(crate = "rmcp::serde")]
struct EvaluateParams {
    expression: String,
    context: Map<String, Value>,
}

#[derive(Serialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
#[serde(crate = "rmcp::serde")]
struct EvaluateResult {
    result: String,
}

#[tool_router]
impl CelTool {
    pub fn new(eval_tx: mpsc::Sender<EvalRequest>) -> Self {
        Self {
            eval_tx,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Evaluates a Common Expression Language (CEL) expression.")]
    async fn evaluate(
        &self,
        params: Parameters<EvaluateParams>,
    ) -> Result<Json<EvaluateResult>, ErrorData> {
        tracing::info!(
            "CelTool::evaluate called with expression: {:?}",
            params.0.expression
        );
        let (responder, receiver) = oneshot::channel();

        let request = EvalRequest {
            expression: params.0.expression,
            context: Value::Object(params.0.context),
            responder,
        };

        if self.eval_tx.send(request).await.is_err() {
            tracing::error!("Failed to send evaluation request to service, service is down.");
            return Err(ErrorData::internal_error("Evaluator service is down", None));
        }

        match receiver.await {
            Ok(Ok(value)) => {
                tracing::info!("Evaluation successful, returning result.");
                Ok(Json(EvaluateResult {
                    result: serde_json::to_string(&value).unwrap_or_else(|_| value.to_string()),
                }))
            }
            Ok(Err(e)) => {
                tracing::error!("Evaluation failed: {}", e);
                Err(ErrorData::internal_error(e, None))
            }
            Err(_) => {
                tracing::error!("Failed to receive response from evaluator service.");
                Err(ErrorData::internal_error(
                    "Failed to receive response from evaluator",
                    None,
                ))
            }
        }
    }
}

#[tool_handler]
impl ServerHandler for CelTool {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: Default::default(),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some("This server provides a single tool to evaluate Common Expression Language (CEL) expressions.".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::serde_json::json;

    #[test]
    fn test_real_evaluate_addition() {
        let context = json!({});
        let response = real_evaluate("1+2", &context);
        assert_eq!(response, Ok(Value::from(3)));
    }

    #[test]
    fn test_real_evaluate_with_context() {
        let context = json!({
            "a": 5,
            "b": 10
        });
        let response = real_evaluate("a * b", &context);
        assert_eq!(response, Ok(Value::from(50)));
    }

    #[test]
    fn test_real_evaluate_string_concat() {
        let context = json!({
            "name": "World"
        });
        let response = real_evaluate("'Hello, ' + name", &context);
        assert_eq!(response, Ok(Value::from("Hello, World")));
    }

    #[test]
    fn test_real_evaluate_compilation_error() {
        let context = json!({});
        let response = real_evaluate("1 +/ 2", &context);
        assert!(response.is_err());
        assert!(response.unwrap_err().contains("compile"));
    }
}
