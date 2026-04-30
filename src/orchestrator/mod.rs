pub mod cancel_order_saga;
pub mod checkout_saga;
pub mod confirm_order_saga;
pub mod payment_saga;

use async_trait::async_trait;
use tracing::{error, info, warn};

use crate::error::AppError;

// SAGA TRAIT
#[async_trait]
pub trait SagaStep: Send + Sync {
    type Context: Send;

    async fn execute(&self, ctx: &mut Self::Context) -> Result<(), AppError>;

    async fn compensate(&self, ctx: &mut Self::Context) -> Result<(), AppError>;

    fn name(&self) -> &'static str;
}

// SAGA ORCHESTRATOR
pub struct SagaOrchestrator<'a, Ctx> {
    saga_name: &'static str,
    steps: Vec<Box<dyn SagaStep<Context = Ctx> + 'a>>,
}

impl<'a, Ctx: Send> SagaOrchestrator<'a, Ctx> {
    pub fn new(saga_name: &'static str) -> Self {
        Self {
            saga_name,
            steps: vec![],
        }
    }

    pub fn step(mut self, s: impl SagaStep<Context = Ctx> + 'a) -> Self {
        self.steps.push(Box::new(s));
        self
    }

    pub async fn run(&self, ctx: &mut Ctx) -> Result<(), AppError> {
        info!(
            "🎬 [saga:{}] memulai — {} steps",
            self.saga_name,
            self.steps.len()
        );

        let mut executed: Vec<usize> = Vec::new();

        for (i, step) in self.steps.iter().enumerate() {
            info!(
                "▶️  [saga:{}] step {}/{} — '{}'",
                self.saga_name,
                i + 1,
                self.steps.len(),
                step.name()
            );

            match step.execute(ctx).await {
                Ok(_) => {
                    info!("✅ [saga:{}] step '{}' sukses", self.saga_name, step.name());
                    executed.push(i);
                }
                Err(e) => {
                    error!(
                        "❌ [saga:{}] step '{}' GAGAL: {:?} — memulai rollback ({} step yang perlu dikompensasi)",
                        self.saga_name,
                        step.name(),
                        e,
                        executed.len()
                    );
                    self.rollback(ctx, &executed).await;
                    return Err(e);
                }
            }
        }

        info!("🏁 [saga:{}] semua step sukses", self.saga_name);
        Ok(())
    }

    async fn rollback(&self, ctx: &mut Ctx, executed: &[usize]) {
        warn!(
            "⏪ [saga:{}] rollback dimulai — mengkompensasi {} step",
            self.saga_name,
            executed.len()
        );

        for &i in executed.iter().rev() {
            let step = &self.steps[i];
            warn!(
                "↩️  [saga:{}] compensating step '{}'",
                self.saga_name,
                step.name()
            );

            match step.compensate(ctx).await {
                Ok(_) => {
                    info!(
                        "✅ [saga:{}] kompensasi '{}' berhasil",
                        self.saga_name,
                        step.name()
                    );
                }
                Err(e) => {
                    error!(
                        "🚨 [saga:{}] KOMPENSASI '{}' GAGAL: {:?}. \
                         STATE INCONSISTENT — butuh intervensi manual!",
                        self.saga_name,
                        step.name(),
                        e
                    );
                }
            }
        }

        warn!("⏪ [saga:{}] rollback selesai", self.saga_name);
    }
}
