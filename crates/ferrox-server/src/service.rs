use tonic::{Request, Response, Status};

use ferrox::cp::solve_cp;
use ferrox::lp::solve_lp;
use ferrox::mip::solve_mip;

use crate::convert::{
    cp_req_from_proto, cp_resp_to_proto, lp_req_from_proto, lp_resp_to_proto, mip_req_from_proto,
    mip_resp_to_proto,
};
use crate::proto::ferrox::v1::ferrox_solver_server::FerroxSolver;
use crate::proto::ferrox::v1::{
    SolveCpRequest, SolveCpResponse, SolveLpRequest, SolveLpResponse, SolveMipRequest,
    SolveMipResponse,
};

#[derive(Default)]
pub struct FerroxSolverService;

#[tonic::async_trait]
impl FerroxSolver for FerroxSolverService {
    async fn solve_cp(
        &self,
        request: Request<SolveCpRequest>,
    ) -> Result<Response<SolveCpResponse>, Status> {
        let req = cp_req_from_proto(request.into_inner())?;
        let plan = solve_cp(&req);
        Ok(Response::new(cp_resp_to_proto(plan)))
    }

    async fn solve_lp(
        &self,
        request: Request<SolveLpRequest>,
    ) -> Result<Response<SolveLpResponse>, Status> {
        let req = lp_req_from_proto(request.into_inner())?;
        let plan = solve_lp(&req);
        Ok(Response::new(lp_resp_to_proto(plan)))
    }

    async fn solve_mip(
        &self,
        request: Request<SolveMipRequest>,
    ) -> Result<Response<SolveMipResponse>, Status> {
        let req = mip_req_from_proto(request.into_inner())?;
        let plan = solve_mip(&req);
        Ok(Response::new(mip_resp_to_proto(plan)))
    }
}
