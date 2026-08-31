//! 泛型数字建模演示（构建级）
//! Generic number modeling demo (build-only)

use std::error::Error;
use std::fmt::Debug;

use ospf_rust_core::model::MetaModel;

fn build_generic_model<V>(name: &str) -> MetaModel<V>
where
    V: Clone + Debug + Send + Sync + 'static,
{
    let mut model = MetaModel::<V>::new(name);
    model.maximize();
    model
}

/// 仅验证泛型主链可构建，不依赖具体求解器。
/// Only validates generic modeling chain can be built without a concrete solver.
pub fn run() -> Result<(), Box<dyn Error>> {
    let model_f64 = build_generic_model::<f64>("core-generic-number-f64");
    let mechanism_f64 = model_f64.try_to_mechanism_model()?;
    println!(
        "generic number demo built: name={}, vars={}, constraints={}",
        mechanism_f64.name,
        mechanism_f64.num_variables(),
        mechanism_f64.num_constraints()
    );

    Ok(())
}
