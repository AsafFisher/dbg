use core::{CallCmd, ReadCmd, WriteCmd, CMD};

use serde_generate;
use serde_reflection::{Tracer, TracerConfig};
fn main() {
    let mut tracer = Tracer::new(TracerConfig::default());
    tracer.trace_simple_type::<CMD>().unwrap();
    tracer.trace_simple_type::<ReadCmd>().unwrap();
    tracer.trace_simple_type::<WriteCmd>().unwrap();
    tracer.trace_simple_type::<CallCmd>().unwrap();
    let registry = tracer.registry().unwrap();
    let data = serde_yaml::to_string(&registry).unwrap();
    std::fs::write("./output.yml", data).unwrap();
    // let mut source = Vec::new();
    // let config = serde_generate::CodeGeneratorConfig::new("testing".to_string())
    // .with_encodings(vec![serde_generate::Encoding::Bincode]);
    // let generator = serde_generate::python3::CodeGenerator::new(&config);
    // generator.output(&mut source, &registry)?;
}
