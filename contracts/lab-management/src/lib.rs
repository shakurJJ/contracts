#![no_std]
#![allow(deprecated)]

use soroban_sdk::{
    Address, BytesN, Env, String, Symbol, Vec, contract, contracterror, contractimpl, contracttype,
    panic_with_error,
};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    NotFound = 1,
    Unauthorized = 2,
    QCFieldFailed = 4,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct TestResult {
    pub test_code: String,
    pub test_name: String,
    pub value: String,
    pub unit: String,
    pub reference_range: String,
    pub is_abnormal: bool,
    pub abnormal_flag: Option<Symbol>,
}

#[contracttype]
pub struct LabOrder {
    pub provider_id: Address,
    pub patient_id: Address,
    pub lab_id: Option<Address>,
    pub test_panel: Vec<String>,
    pub status: Symbol,
    pub results_hash: Option<BytesN<32>>,
    pub quality_control_passed: bool,
}

#[contracttype]
pub struct OrderRequest {
    pub test_panel: Vec<String>,
    pub priority: Symbol,
    pub clinical_info_hash: BytesN<32>,
    pub fasting_required: bool,
    pub collection_date: Option<u64>,
}

#[contract]
pub struct LabManagementContract;

#[contractimpl]
impl LabManagementContract {
    /// Validates QC check results before any state mutations occur
    /// Returns Ok if validation passes, Err if validation fails
    fn validate_qc_results(qc_passed: bool, results_summary: &Vec<TestResult>) -> Result<(), Error> {
        // Check QC status
        if !qc_passed {
            return Err(Error::QCFieldFailed);
        }

        // Check results are not empty
        if results_summary.is_empty() {
            return Err(Error::NotFound);
        }

        Ok(())
    }

    pub fn order_lab_test(
        env: Env,
        provider_id: Address,
        patient_id: Address,
        req: OrderRequest,
    ) -> u64 {
        provider_id.require_auth();
        let counter_key = Symbol::new(&env, "LAB_ID");
        let id: u64 = env
            .storage()
            .instance()
            .get::<_, u64>(&counter_key)
            .unwrap_or(0);
        env.storage().instance().set(&counter_key, &(id + 1));

        let order = LabOrder {
            provider_id,
            patient_id,
            lab_id: None,
            test_panel: req.test_panel,
            status: Symbol::new(&env, "Ordered"),
            results_hash: None,
            quality_control_passed: false,
        };

        env.storage().persistent().set(&id, &order);
        id
    }

    pub fn assign_lab(env: Env, order_id: u64, lab_id: Address, _eta: u64) {
        let mut order: LabOrder = env
            .storage()
            .persistent()
            .get(&order_id)
            .unwrap_or_else(|| panic_with_error!(&env, Error::NotFound));
        order.lab_id = Some(lab_id);
        order.status = Symbol::new(&env, "Assigned");
        env.storage().persistent().set(&order_id, &order);
    }

    pub fn submit_results(
        env: Env,
        order_id: u64,
        lab_id: Address,
        results_hash: BytesN<32>,
        results_summary: Vec<TestResult>,
        qc_passed: bool,
    ) {
        lab_id.require_auth();

        // VALIDATION PHASE: All validations must pass before any storage writes
        // This ensures no partial state mutations on failure

        // 1. Verify order exists
        let mut order: LabOrder = env
            .storage()
            .persistent()
            .get(&order_id)
            .unwrap_or_else(|| panic_with_error!(&env, Error::NotFound));

        // 2. Perform QC validation (BEFORE any mutations)
        // All checks must pass before proceeding to mutation phase
        Self::validate_qc_results(qc_passed, &results_summary)
            .unwrap_or_else(|e| panic_with_error!(&env, e));

        // MUTATION PHASE: All state changes after validations have passed
        // Only reached if all validations succeeded

        order.results_hash = Some(results_hash);
        order.quality_control_passed = qc_passed;
        order.status = Symbol::new(&env, "Completed");

        // Publish event before final storage write
        env.events().publish(
            (
                Symbol::new(&env, "LAB"),
                Symbol::new(&env, "RESULT"),
                order.patient_id.clone(),
            ),
            results_summary,
        );

        // Final transactional write (only reached if all validations passed)
        env.storage().persistent().set(&order_id, &order);
    }

    pub fn flag_critical_value(
        env: Env,
        order_id: u64,
        lab_id: Address,
        test_code: String,
        val: String,
    ) {
        lab_id.require_auth();
        // Event for critical alerting
        env.events()
            .publish((Symbol::new(&env, "CRITICAL"), order_id), (test_code, val));
    }
}
mod test;
