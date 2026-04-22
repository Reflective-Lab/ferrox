#include "highs_wrapper.h"
#include "Highs.h"

struct HighsHandle {
    Highs highs;
    int   num_cols = 0;
};

extern "C" {

HighsHandle* highs_create() {
    auto* h = new HighsHandle{};
    // Suppress HiGHS output — ferrox callers use tracing instead
    h->highs.setOptionValue("output_flag", false);
    return h;
}

void highs_destroy(HighsHandle* h) { delete h; }

HighsReturnStatus highs_add_col(HighsHandle* h, double cost, double lb, double ub) {
    auto s = h->highs.addCol(cost, lb, ub, 0, nullptr, nullptr);
    if (s == HighsStatus::kOk) h->num_cols++;
    return s == HighsStatus::kOk ? HIGHS_kOk : HIGHS_kError;
}

HighsReturnStatus highs_add_row(HighsHandle* h, double lb, double ub,
                                 int num_nz, const int* idx, const double* val) {
    auto s = h->highs.addRow(lb, ub, num_nz, idx, val);
    return s == HighsStatus::kOk ? HIGHS_kOk : HIGHS_kError;
}

HighsReturnStatus highs_change_col_integer_type(HighsHandle* h, int col, int is_integer) {
    auto vtype = is_integer ? HighsVarType::kInteger : HighsVarType::kContinuous;
    auto s = h->highs.changeColIntegrality(col, vtype);
    return s == HighsStatus::kOk ? HIGHS_kOk : HIGHS_kError;
}

HighsReturnStatus highs_set_time_limit(HighsHandle* h, double seconds) {
    auto s = h->highs.setOptionValue("time_limit", seconds);
    return s == HighsStatus::kOk ? HIGHS_kOk : HIGHS_kError;
}

HighsReturnStatus highs_set_mip_rel_gap(HighsHandle* h, double gap) {
    auto s = h->highs.setOptionValue("mip_rel_gap", gap);
    return s == HighsStatus::kOk ? HIGHS_kOk : HIGHS_kError;
}

HighsReturnStatus highs_run(HighsHandle* h) {
    auto s = h->highs.run();
    return s == HighsStatus::kOk ? HIGHS_kOk : HIGHS_kError;
}

HighsModelStatus highs_get_model_status(HighsHandle* h) {
    auto ms = h->highs.getModelStatus();
    switch (ms) {
        case HighsModelStatus::kOptimal:          return HIGHS_MODEL_STATUS_OPTIMAL;
        case HighsModelStatus::kInfeasible:       return HIGHS_MODEL_STATUS_INFEASIBLE;
        case HighsModelStatus::kUnbounded:        return HIGHS_MODEL_STATUS_UNBOUNDED;
        case HighsModelStatus::kSolutionLimit:    return HIGHS_MODEL_STATUS_SOLUTION_LIMIT;
        case HighsModelStatus::kTimeLimit:        return HIGHS_MODEL_STATUS_TIME_LIMIT;
        default:                                  return HIGHS_MODEL_STATUS_NOT_SET;
    }
}

double highs_get_objective_value(HighsHandle* h) {
    return h->highs.getInfoValue<double>("objective_function_value").second;
}

double highs_get_col_value(HighsHandle* h, int col) {
    const HighsSolution& sol = h->highs.getSolution();
    return sol.col_value[col];
}

double highs_get_mip_gap(HighsHandle* h) {
    return h->highs.getInfoValue<double>("mip_gap").second;
}

} // extern "C"
