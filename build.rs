fn main() {
    let out_dir_env = std::env::var_os("OUT_DIR").unwrap();
    let out_dir = std::path::Path::new(&out_dir_env);
    protobuf_codegen::Codegen::new()
        .out_dir(out_dir)
        .protoc()
        .includes(["proto"])
        .input("proto/common.proto")
        .input("proto/control.proto")
        .input("proto/input.proto")
        .input("proto/media.proto")
        .input("proto/navigation.proto")
        .input("proto/playback.proto")
        .input("proto/sensors.proto")
        .cargo_out_dir("protobuf")
        .run_from_script();
}