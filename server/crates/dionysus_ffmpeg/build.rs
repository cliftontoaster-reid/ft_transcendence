use std::fs;
use std::path::PathBuf;

fn find_protos(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
  for entry in fs::read_dir(dir).unwrap() {
    let p = entry.unwrap().path();
    if p.is_dir() {
      find_protos(&p, out);
    } else if p.extension().is_some_and(|e| e == "proto") {
      out.push(p);
    }
  }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
  let proto_root = manifest.join("../../../proto").canonicalize()?;
  let googleapis = manifest.join("../../../third_party/googleapis");
  let mut includes = vec![proto_root.clone(), googleapis];
  if let Some(include) = std::env::var_os("PROTOC_INCLUDE") {
    includes.push(PathBuf::from(include));
  }
  for include in ["/usr/include", "/usr/local/include"] {
    let path = PathBuf::from(include);
    if path.is_dir() {
      includes.push(path);
    }
  }

  let mut protos = Vec::new();
  find_protos(&proto_root, &mut protos);
  protos.sort();

  tonic_prost_build::configure()
    .build_server(true)
    .compile_protos(&protos, &includes)?;

  for p in &protos {
    println!("cargo:rerun-if-changed={}", p.display());
  }
  Ok(())
}
