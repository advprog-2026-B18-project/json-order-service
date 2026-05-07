use async_trait::async_trait;

use crate::error::AppError;
use crate::orchestrator::{SagaOrchestrator, SagaStep};

// ──────────────────────────────────────────────────────────────
// Helper stubs
// ──────────────────────────────────────────────────────────────

pub struct Ctx {
    pub log: Vec<String>,
}

struct OkStep(pub &'static str);

#[async_trait]
impl SagaStep for OkStep {
    type Context = Ctx;

    async fn execute(&self, ctx: &mut Ctx) -> Result<(), AppError> {
        ctx.log.push(format!("execute:{}", self.0));
        Ok(())
    }

    async fn compensate(&self, ctx: &mut Ctx) -> Result<(), AppError> {
        ctx.log.push(format!("compensate:{}", self.0));
        Ok(())
    }

    fn name(&self) -> &'static str {
        self.0
    }
}

struct FailStep(pub &'static str);

#[async_trait]
impl SagaStep for FailStep {
    type Context = Ctx;

    async fn execute(&self, _ctx: &mut Ctx) -> Result<(), AppError> {
        Err(AppError::Internal)
    }

    async fn compensate(&self, ctx: &mut Ctx) -> Result<(), AppError> {
        ctx.log.push(format!("compensate:{}", self.0));
        Ok(())
    }

    fn name(&self) -> &'static str {
        self.0
    }
}

struct FailCompensateStep(pub &'static str);

#[async_trait]
impl SagaStep for FailCompensateStep {
    type Context = Ctx;

    async fn execute(&self, ctx: &mut Ctx) -> Result<(), AppError> {
        ctx.log.push(format!("execute:{}", self.0));
        Ok(())
    }

    async fn compensate(&self, _ctx: &mut Ctx) -> Result<(), AppError> {
        Err(AppError::Internal)
    }

    fn name(&self) -> &'static str {
        self.0
    }
}

// ──────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn saga_sukses_semua_step_dieksekusi() {
    let mut ctx = Ctx { log: vec![] };
    let saga = SagaOrchestrator::new("test")
        .step(OkStep("step1"))
        .step(OkStep("step2"))
        .step(OkStep("step3"));

    let result = saga.run(&mut ctx).await;

    assert!(result.is_ok());
    assert_eq!(
        ctx.log,
        vec!["execute:step1", "execute:step2", "execute:step3"]
    );
}

#[tokio::test]
async fn saga_tanpa_step_sukses() {
    let mut ctx = Ctx { log: vec![] };
    let saga: SagaOrchestrator<Ctx> = SagaOrchestrator::new("empty");

    let result = saga.run(&mut ctx).await;

    assert!(result.is_ok());
    assert!(ctx.log.is_empty());
}

#[tokio::test]
async fn saga_step_pertama_gagal_tidak_ada_kompensasi() {
    let mut ctx = Ctx { log: vec![] };
    let saga = SagaOrchestrator::new("test")
        .step(FailStep("step1"))
        .step(OkStep("step2"));

    let result = saga.run(&mut ctx).await;

    assert!(result.is_err());
    // step1 gagal di execute, tidak ada yang perlu dikompensasi
    assert!(ctx.log.is_empty());
}

#[tokio::test]
async fn saga_step_kedua_gagal_kompensasi_step_pertama() {
    let mut ctx = Ctx { log: vec![] };
    let saga = SagaOrchestrator::new("test")
        .step(OkStep("step1"))
        .step(FailStep("step2"))
        .step(OkStep("step3"));

    let result = saga.run(&mut ctx).await;

    assert!(result.is_err());
    // step1 ok, step2 fail → kompensasi step1
    assert!(ctx.log.contains(&"execute:step1".to_string()));
    assert!(ctx.log.contains(&"compensate:step1".to_string()));
    // step3 tidak pernah dieksekusi
    assert!(!ctx.log.iter().any(|s| s.contains("step3")));
}

#[tokio::test]
async fn saga_step_ketiga_gagal_kompensasi_step_dua_dan_satu_secara_terbalik() {
    let mut ctx = Ctx { log: vec![] };
    let saga = SagaOrchestrator::new("test")
        .step(OkStep("step1"))
        .step(OkStep("step2"))
        .step(FailStep("step3"));

    let result = saga.run(&mut ctx).await;

    assert!(result.is_err());
    // Urutan kompensasi harus terbalik: step2, step1
    let compensate_positions: Vec<_> = ctx
        .log
        .iter()
        .enumerate()
        .filter(|(_, s)| s.starts_with("compensate:"))
        .collect();
    assert_eq!(compensate_positions.len(), 2);
    assert_eq!(compensate_positions[0].1, "compensate:step2");
    assert_eq!(compensate_positions[1].1, "compensate:step1");
}

#[tokio::test]
async fn saga_rollback_kompensasi_gagal_tidak_stop_rollback() {
    // Kompensasi yang gagal tidak boleh menghentikan proses rollback step lain
    let mut ctx = Ctx { log: vec![] };
    let saga = SagaOrchestrator::new("test")
        .step(OkStep("step1"))
        .step(FailCompensateStep("step2"))
        .step(FailStep("step3"));

    let result = saga.run(&mut ctx).await;

    // Saga tetap return error dari step yang gagal
    assert!(result.is_err());
    // step1 dan step2 berhasil dieksekusi
    assert!(ctx.log.contains(&"execute:step1".to_string()));
    assert!(ctx.log.contains(&"execute:step2".to_string()));
}

#[tokio::test]
async fn saga_satu_step_sukses() {
    let mut ctx = Ctx { log: vec![] };
    let saga = SagaOrchestrator::new("single").step(OkStep("only"));

    let result = saga.run(&mut ctx).await;

    assert!(result.is_ok());
    assert_eq!(ctx.log, vec!["execute:only"]);
}

#[tokio::test]
async fn saga_satu_step_gagal_tidak_ada_kompensasi() {
    let mut ctx = Ctx { log: vec![] };
    let saga = SagaOrchestrator::new("single").step(FailStep("only"));

    let result = saga.run(&mut ctx).await;

    assert!(result.is_err());
    // Tidak ada step yang dieksekusi sebelumnya → tidak ada kompensasi
    assert!(ctx.log.is_empty());
}
