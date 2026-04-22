#include "wrapper.h"

#include <cstring>
#include <memory>
#include <vector>

#include "ortools/sat/cp_model.h"
#include "ortools/sat/cp_model_solver.h"
#include "ortools/sat/sat_parameters.pb.h"
#include "ortools/linear_solver/linear_solver.h"

using namespace operations_research;
using namespace operations_research::sat;

// ── CP-SAT ────────────────────────────────────────────────────────────────────

// We store the CpModelBuilder as an opaque heap object.
// The C ABI uses a forward-declared struct tag; the real type lives here.
struct CpModelBuilder {
    sat::CpModelBuilder builder;
};

struct CpSolverResponse {
    sat::CpSolverResponse response;
};

extern "C" {

CpModelBuilder* cpmodel_new() {
    return new CpModelBuilder{};
}

void cpmodel_free(CpModelBuilder* m) {
    delete m;
}

int32_t cpmodel_new_int_var(CpModelBuilder* m, int64_t lb, int64_t ub, const char* name) {
    auto v = m->builder.NewIntVar(Domain(lb, ub));
    if (name && name[0]) v = v.WithName(name);
    return v.index();
}

int32_t cpmodel_new_bool_var(CpModelBuilder* m, const char* name) {
    auto v = m->builder.NewBoolVar();
    if (name && name[0]) v = v.WithName(name);
    return v.index();
}

static LinearExpr make_expr(const int32_t* idx, const int64_t* c, size_t n) {
    LinearExpr expr;
    for (size_t i = 0; i < n; ++i)
        expr += LinearExpr::Term(IntVar(idx[i]), c[i]);
    return expr;
}

void cpmodel_add_linear_le(CpModelBuilder* m, const int32_t* idx,
                            const int64_t* c, size_t n, int64_t rhs) {
    m->builder.AddLessOrEqual(make_expr(idx, c, n), rhs);
}

void cpmodel_add_linear_ge(CpModelBuilder* m, const int32_t* idx,
                            const int64_t* c, size_t n, int64_t rhs) {
    m->builder.AddGreaterOrEqual(make_expr(idx, c, n), rhs);
}

void cpmodel_add_linear_eq(CpModelBuilder* m, const int32_t* idx,
                            const int64_t* c, size_t n, int64_t rhs) {
    m->builder.AddEquality(make_expr(idx, c, n), rhs);
}

void cpmodel_add_all_different(CpModelBuilder* m, const int32_t* idx, size_t n) {
    std::vector<IntVar> vars;
    vars.reserve(n);
    for (size_t i = 0; i < n; ++i) vars.push_back(IntVar(idx[i]));
    m->builder.AddAllDifferent(vars);
}

void cpmodel_minimize(CpModelBuilder* m, const int32_t* idx, const int64_t* c, size_t n) {
    m->builder.Minimize(make_expr(idx, c, n));
}

void cpmodel_maximize(CpModelBuilder* m, const int32_t* idx, const int64_t* c, size_t n) {
    m->builder.Maximize(make_expr(idx, c, n));
}

CpSolverResponse* cpmodel_solve(CpModelBuilder* m, double time_limit) {
    SatParameters params;
    if (time_limit > 0.0) params.set_max_time_in_seconds(time_limit);
    params.set_num_search_workers(1);

    auto* r = new CpSolverResponse{};
    r->response = SolveWithParameters(m->builder.Build(), params);
    return r;
}

OrtoolsStatus cpresponse_status(const CpSolverResponse* r) {
    return static_cast<OrtoolsStatus>(r->response.status());
}

int64_t cpresponse_objective_value(const CpSolverResponse* r) {
    return static_cast<int64_t>(r->response.objective_value());
}

int64_t cpresponse_value(const CpSolverResponse* r, int32_t var_index) {
    return SolutionIntegerValue(r->response, IntVar(var_index));
}

double cpresponse_wall_time(const CpSolverResponse* r) {
    return r->response.wall_time();
}

void cpresponse_free(CpSolverResponse* r) {
    delete r;
}

// ── GLOP ──────────────────────────────────────────────────────────────────────

struct MpSolver {
    std::unique_ptr<MPSolver> solver;
    std::vector<MPVariable*>   vars;
    std::vector<MPConstraint*> constraints;
};

MpSolver* mpsolver_new(const char* name, LpSolverType /*type*/) {
    auto* s = new MpSolver{};
    s->solver = std::unique_ptr<MPSolver>(MPSolver::CreateSolver("GLOP"));
    if (!s->solver) { delete s; return nullptr; }
    return s;
}

void mpsolver_free(MpSolver* s) { delete s; }

int32_t mpsolver_num_var(MpSolver* s, double lb, double ub, const char* name) {
    auto* v = s->solver->MakeNumVar(lb, ub, name ? name : "");
    auto idx = static_cast<int32_t>(s->vars.size());
    s->vars.push_back(v);
    return idx;
}

int32_t mpsolver_int_var(MpSolver* s, double lb, double ub, const char* name) {
    auto* v = s->solver->MakeIntVar(lb, ub, name ? name : "");
    auto idx = static_cast<int32_t>(s->vars.size());
    s->vars.push_back(v);
    return idx;
}

int32_t mpsolver_bool_var(MpSolver* s, const char* name) {
    auto* v = s->solver->MakeBoolVar(name ? name : "");
    auto idx = static_cast<int32_t>(s->vars.size());
    s->vars.push_back(v);
    return idx;
}

int32_t mpsolver_add_constraint(MpSolver* s, double lb, double ub, const char* name) {
    auto* c = s->solver->MakeRowConstraint(lb, ub, name ? name : "");
    auto idx = static_cast<int32_t>(s->constraints.size());
    s->constraints.push_back(c);
    return idx;
}

void mpsolver_set_constraint_coeff(MpSolver* s, int32_t ci, int32_t vi, double coeff) {
    s->constraints[ci]->SetCoefficient(s->vars[vi], coeff);
}

void mpsolver_set_objective_coeff(MpSolver* s, int32_t vi, double coeff) {
    s->solver->MutableObjective()->SetCoefficient(s->vars[vi], coeff);
}

void mpsolver_minimize(MpSolver* s) { s->solver->MutableObjective()->SetMinimization(); }
void mpsolver_maximize(MpSolver* s) { s->solver->MutableObjective()->SetMaximization(); }

OrtoolsStatus mpsolver_solve(MpSolver* s) {
    auto result = s->solver->Solve();
    switch (result) {
        case MPSolver::OPTIMAL:    return ORTOOLS_OPTIMAL;
        case MPSolver::FEASIBLE:   return ORTOOLS_FEASIBLE;
        case MPSolver::INFEASIBLE: return ORTOOLS_INFEASIBLE;
        case MPSolver::UNBOUNDED:  return ORTOOLS_UNBOUNDED;
        default:                   return ORTOOLS_ERROR;
    }
}

double mpsolver_objective_value(const MpSolver* s) {
    return s->solver->Objective().Value();
}

double mpsolver_var_value(const MpSolver* s, int32_t vi) {
    return s->vars[vi]->solution_value();
}

} // extern "C"
