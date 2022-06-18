fn main() {
    tonic_build::configure()
        .build_server(false)
        .compile(
            &["proto/jaeger/query.proto"],
            &["proto"], // specify the root location to search proto dependencies
        )
        .unwrap();
}
