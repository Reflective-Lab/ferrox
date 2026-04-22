#pragma once
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef enum {
    ORTOOLS_UNKNOWN       = 0,
    ORTOOLS_OPTIMAL       = 1,
    ORTOOLS_FEASIBLE      = 2,
    ORTOOLS_INFEASIBLE    = 3,
    ORTOOLS_UNBOUNDED     = 4,
    ORTOOLS_MODEL_INVALID = 5,
    ORTOOLS_ERROR         = 6,
} OrtoolsStatus;

/* ── CP-SAT ─────────────────────────────────────────────────────────────── */
typedef struct CpModelBuilder   CpModelBuilder;
typedef struct CpSolverResponse CpSolverResponse;

CpModelBuilder*   cpmodel_new(void);
void              cpmodel_free(CpModelBuilder* m);
int32_t           cpmodel_new_int_var(CpModelBuilder* m, int64_t lb, int64_t ub, const char* name);
int32_t           cpmodel_new_bool_var(CpModelBuilder* m, const char* name);
void              cpmodel_add_linear_le(CpModelBuilder* m, const int32_t* idx,
                                        const int64_t* c, size_t n, int64_t rhs);
void              cpmodel_add_linear_ge(CpModelBuilder* m, const int32_t* idx,
                                        const int64_t* c, size_t n, int64_t rhs);
void              cpmodel_add_linear_eq(CpModelBuilder* m, const int32_t* idx,
                                        const int64_t* c, size_t n, int64_t rhs);
void              cpmodel_add_all_different(CpModelBuilder* m, const int32_t* idx, size_t n);
void              cpmodel_minimize(CpModelBuilder* m, const int32_t* idx,
                                   const int64_t* c, size_t n);
void              cpmodel_maximize(CpModelBuilder* m, const int32_t* idx,
                                   const int64_t* c, size_t n);
CpSolverResponse* cpmodel_solve(CpModelBuilder* m, double time_limit);
OrtoolsStatus     cpresponse_status(const CpSolverResponse* r);
int64_t           cpresponse_objective_value(const CpSolverResponse* r);
int64_t           cpresponse_value(const CpSolverResponse* r, int32_t var_index);
double            cpresponse_wall_time(const CpSolverResponse* r);
void              cpresponse_free(CpSolverResponse* r);

/* ── GLOP / MP linear solver ─────────────────────────────────────────────── */
typedef enum { LP_GLOP = 0 } LpSolverType;
typedef struct MpSolver MpSolver;

MpSolver*     mpsolver_new(const char* name, LpSolverType type);
void          mpsolver_free(MpSolver* s);
int32_t       mpsolver_num_var(MpSolver* s, double lb, double ub, const char* name);
int32_t       mpsolver_int_var(MpSolver* s, double lb, double ub, const char* name);
int32_t       mpsolver_bool_var(MpSolver* s, const char* name);
int32_t       mpsolver_add_constraint(MpSolver* s, double lb, double ub, const char* name);
void          mpsolver_set_constraint_coeff(MpSolver* s, int32_t ci, int32_t vi, double coeff);
void          mpsolver_set_objective_coeff(MpSolver* s, int32_t vi, double coeff);
void          mpsolver_minimize(MpSolver* s);
void          mpsolver_maximize(MpSolver* s);
OrtoolsStatus mpsolver_solve(MpSolver* s);
double        mpsolver_objective_value(const MpSolver* s);
double        mpsolver_var_value(const MpSolver* s, int32_t vi);

#ifdef __cplusplus
}
#endif
