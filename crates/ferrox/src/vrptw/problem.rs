use serde::{Deserialize, Serialize};

/// A customer to be visited by the vehicle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Customer {
    pub id: usize,
    pub name: String,
    pub x: f64,
    pub y: f64,
    /// Earliest arrival time.
    pub window_open: i64,
    /// Latest arrival time (must arrive by this time).
    pub window_close: i64,
    /// Service duration at this customer.
    pub service_time: i64,
}

impl Customer {
    pub fn travel_to(&self, other: &Customer) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

/// The depot — vehicle starts and ends here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Depot {
    pub x: f64,
    pub y: f64,
    pub ready_time: i64,
    pub due_time: i64,
}

impl Depot {
    pub fn travel_to_customer(&self, c: &Customer) -> f64 {
        let dx = self.x - c.x;
        let dy = self.y - c.y;
        (dx * dx + dy * dy).sqrt()
    }
}

/// Seeded into `ContextKey::Seeds` with id prefix `"vrptw-request:"`.
///
/// Models a single-vehicle TSP with Time Windows (TSPTW).
/// Customers are optional: the objective is to maximise customers visited
/// while respecting time windows and the vehicle's return deadline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VrptwRequest {
    pub id: String,
    pub depot: Depot,
    pub customers: Vec<Customer>,
    #[serde(default = "default_time_limit")]
    pub time_limit_seconds: f64,
}

fn default_time_limit() -> f64 {
    30.0
}

/// One stop in the vehicle's route.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteStop {
    pub customer_id: usize,
    pub customer_name: String,
    pub arrival: i64,
    pub departure: i64,
}

/// Written to `ContextKey::Strategies` with id prefix `"vrptw-plan-<solver>:"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VrptwPlan {
    pub request_id: String,
    /// Ordered stops (depot → customers → depot is implied).
    pub route: Vec<RouteStop>,
    pub customers_total: usize,
    pub customers_visited: usize,
    /// Total travel distance (unscaled, Euclidean).
    pub total_distance: f64,
    /// Time the vehicle returns to depot.
    pub return_time: i64,
    pub solver: String,
    pub status: String,
    pub wall_time_seconds: f64,
}

impl VrptwPlan {
    pub fn visit_ratio(&self) -> f64 {
        if self.customers_total == 0 {
            return 0.0;
        }
        self.customers_visited as f64 / self.customers_total as f64
    }
}
