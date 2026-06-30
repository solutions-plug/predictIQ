use predictiq_api::openapi_spec::ApiDoc;
use utoipa::OpenApi;

fn main() {
    let doc = ApiDoc::openapi();
    let yaml = doc.to_yaml().expect("Failed to serialize OpenAPI spec as YAML");
    print!("{yaml}");
}
