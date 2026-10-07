use anyhow::{Result, ensure};
use std::path::Path;
use usearch::{Index, IndexOptions, MetricKind, ScalarKind};

pub fn normalize(mut v: Vec<f32>, dim: usize) -> Result<Vec<f32>> {
    ensure!(
        [128, 256, 512, 768].contains(&dim) && v.len() >= dim,
        "Invalid embedding dimension"
    );
    v.truncate(dim);
    ensure!(v.iter().all(|x| x.is_finite()), "Non-finite embedding");
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    ensure!(norm > 1e-12, "Zero embedding");
    for x in &mut v {
        *x /= norm;
    }
    Ok(v)
}
pub struct Vectors {
    pub general: Index,
    pub code: Index,
    pub dimensions: usize,
}
impl Vectors {
    pub fn load(&self, path: &Path) -> Result<()> {
        self.general.load(
            path.join("general.usearch")
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid vector path"))?,
        )?;
        self.code.load(
            path.join("code.usearch")
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid vector path"))?,
        )?;
        Ok(())
    }
    pub fn new(dimensions: usize) -> Result<Self> {
        let options = IndexOptions {
            dimensions,
            metric: MetricKind::Cos,
            quantization: ScalarKind::F16,
            connectivity: 16,
            expansion_add: 128,
            expansion_search: 80,
            multi: false,
        };
        Ok(Self {
            general: Index::new(&options)?,
            code: Index::new(&options)?,
            dimensions,
        })
    }
    pub fn add(&self, id: u64, namespace: &str, v: &[f32]) -> Result<()> {
        ensure!(v.len() == self.dimensions, "Vector dimension mismatch");
        let index = if namespace == "code" {
            &self.code
        } else {
            &self.general
        };
        if index.capacity() <= index.size() {
            index.reserve((index.size() + 1024).next_power_of_two())?
        }
        index.add(id, v)?;
        Ok(())
    }
    pub fn remove(&self, id: u64) -> Result<()> {
        self.general.remove(id)?;
        self.code.remove(id)?;
        Ok(())
    }
    pub fn search(&self, v: &[f32], namespace: &str, k: usize) -> Result<Vec<(u64, f32)>> {
        ensure!(v.len() == self.dimensions, "Query dimension mismatch");
        let index = if namespace == "code" {
            &self.code
        } else {
            &self.general
        };
        let result = index.search(v, k)?;
        Ok(result.keys.into_iter().zip(result.distances).collect())
    }
    pub fn save(&self, path: &Path) -> Result<()> {
        std::fs::create_dir_all(path)?;
        for (name, index) in [("general", &self.general), ("code", &self.code)] {
            let temp = path.join(format!("{name}.usearch.tmp"));
            index.save(
                temp.to_str()
                    .ok_or_else(|| anyhow::anyhow!("Invalid vector path"))?,
            )?;
            std::fs::rename(temp, path.join(format!("{name}.usearch")))?
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalizing_mrl() {
        let v = normalize(vec![1.; 768], 256).unwrap();
        assert_eq!(v.len(), 256);
        assert!((v.iter().map(|x| x * x).sum::<f32>() - 1.).abs() < 1e-5);
        assert!(normalize(vec![f32::NAN; 768], 256).is_err());
        assert!(normalize(vec![0.; 768], 256).is_err());
    }
    #[test]
    fn ann_lifecycle() {
        let v = Vectors::new(128).unwrap();
        let mut a = vec![0.; 128];
        a[0] = 1.;
        v.add(7, "general", &a).unwrap();
        assert_eq!(v.search(&a, "general", 10).unwrap()[0].0, 7);
        v.remove(7).unwrap();
        assert!(v.search(&a, "general", 10).unwrap().is_empty());
    }
}
