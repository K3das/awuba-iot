use std::io;

fn main() -> io::Result<()> {
    prost_build::compile_protos(&["proto/aos8v1/aruba-iot-nb.proto"], &["proto/aos8v1"])?;
    Ok(())
}
