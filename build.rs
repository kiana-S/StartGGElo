fn main() {
    cynic_codegen::register_schema("startgg")
        .from_sdl_file("schema/startgg.graphql")
        .unwrap()
        .as_default()
        .unwrap();
}
