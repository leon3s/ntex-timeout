use std::io::Write;

use ntex::web;
use futures::StreamExt;

#[web::post("/import")]
pub(crate) async fn import(
  mut payload: web::types::Payload,
  path: web::types::Path<(String, String)>,
) -> web::HttpResponse {
  let name = path.1.to_owned();
  let vm_images_dir = "/tmp".to_owned();
  let filepath = format!("{vm_images_dir}/{name}.img");
  let fp = filepath.clone();
  let mut f = web::block(move || std::fs::File::create(fp)).await.unwrap();
  while let Some(bytes) = payload.next().await {
    let bytes = bytes.unwrap();
    f = web::block(move || f.write_all(&bytes).map(|_| f))
      .await
      .unwrap();
  }
  web::HttpResponse::Ok().into()
}

#[ntex::main]
async fn main() {
  let server = web::HttpServer::new(move || {
    web::App::new()
      // bind config state
      .state(
        web::types::PayloadConfig::new(20_000_000_000), // <- limit size of the payload
      )
      // Default logger middleware
      .wrap(web::middleware::Logger::default())
      // Set Json body max size
      .state(web::types::JsonConfig::default().limit(20_000_000))
      .service(import)
  });
  server.run().await.unwrap();
}

#[cfg(test)]
mod tests {

  use std::path::Path;

  use futures::StreamExt;
  use tokio_util::codec;
  use ntex::web;

  use super::import;

  #[ntex::test]
  async fn test_upload() {
    let client = web::test::server(move || {
      web::App::new()
        // bind config state
        .state(
          web::types::PayloadConfig::new(20_000_000_000), // <- limit size of the payload
        )
        // Default logger middleware
        .wrap(web::middleware::Logger::default())
        // Set Json body max size
        .state(web::types::JsonConfig::default().limit(20_000_000))
        .service(import)
    });

    let file_path = "./jammy-server-cloudimg-amd64.img".to_owned();
    let fp = Path::new(&file_path).canonicalize().unwrap();
    let file = tokio::fs::File::open(&fp).await.unwrap();
    // Get file size
    let byte_stream = codec::FramedRead::new(file, codec::BytesCodec::new())
      .map(move |r| {
        let r = r?;
        let bytes = ntex::util::Bytes::from_iter(r.freeze().to_vec());
        Ok::<ntex::util::Bytes, std::io::Error>(bytes)
      });

    client
      .post("/import")
      .send_stream(byte_stream)
      .await
      .unwrap();
  }
}
