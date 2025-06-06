use crate::{loader::loader::GModel2, scene::scene::GScene2};

impl GScene2 {
    pub fn print_transforms(&self) {
        let mut lt = self.instance_data.local_transform_data.iter();
        for (i, m) in self.models.iter().enumerate() {
            println!("MODEL {i} ---------------------------------------");
            for (idx, _) in m.mesh_instances.iter().enumerate() {
                println!("MESH {idx} ----------------------------------");
                for _ in 0..m.mesh_instances[idx] {
                    let maybet = lt.next();
                    if let Some(t) = maybet {
                        println!("            {t:?}");
                    }
                }
            }
        }
    }
}

pub(super) fn calculate_model_mesh_offsets(
    models: &Vec<GModel2>,
    model_instances: &Vec<usize>,
) -> Vec<usize> {
    let mut model_mesh_offsets = Vec::with_capacity(models.len());
    let mut sum = 0;
    for (idx, model) in models.iter().enumerate() {
        model_mesh_offsets.push(sum);
        sum += (model.mesh_instances.iter().sum::<u32>() as usize) * model_instances[idx];
    }
    model_mesh_offsets
}
