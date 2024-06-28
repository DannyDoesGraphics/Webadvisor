extern crate core;

use std::collections::{VecDeque};
use std::future::Future;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use anyhow::{anyhow, Result};
use headless_chrome::{Browser, Tab};
use petgraph::data::Build;
use petgraph::prelude::*;
use tokio::time;
use tokio::time::sleep;

use crate::task::Task;

mod browser;
mod microsoft;
pub mod task;
pub mod wa;

pub struct ContextInner {
    pub browser: Browser,
    pub tab: Arc<Tab>,
}

#[derive(Clone, Default)]
pub struct Context {
    pub inner: Arc<RwLock<Option<ContextInner>>>,
}

#[derive(Copy, Clone, PartialEq)]
pub enum TaskEdge {
    Outgoing,
    Incoming,
}

fn ensure_bidirectional_edges(graph: &mut DiGraph<Task, TaskEdge>) {
    let mut edges_to_add = Vec::new();
    for edge in graph.edge_references() {
        let (source, target) = (edge.source(), edge.target());
        if graph.find_edge(target, source).is_none() {
            let reverse_edge_type = match edge.weight() {
                TaskEdge::Outgoing => TaskEdge::Incoming,
                TaskEdge::Incoming => TaskEdge::Outgoing,
            };

            edges_to_add.push((target, source, reverse_edge_type));
        }
    }
    for (source, target, edge_type) in edges_to_add {
        graph.add_edge(source, target, edge_type);
    }
}

async fn run_with_timeout<F, T>(future: F, timeout: Duration) -> Result<T>
where
    F: Future<Output = Result<T>>,
{
    match time::timeout(timeout, future).await {
        Ok(result) => result.map_err(|e| anyhow::anyhow!(e)),
        Err(_) => Err(anyhow::anyhow!("timeout")),
    }
}

async fn execute_task(graph: &DiGraph<Task, TaskEdge>, ctx: Context, node: NodeIndex) -> bool {
    let task = &graph[node];

    let pre_condition_timeout = Duration::from_secs(5);
    let task_timeout = Duration::from_secs(20);

    // Retry precondition until timeout
    let pre_condition_future = async {
        let start = time::Instant::now();
        loop {
            if start.elapsed() >= pre_condition_timeout {
                return Err(anyhow!("Precondition check timed out"));
            }

            match (task.pre_condition)(ctx.clone()).await {
                Ok(true) => return Ok(true),
                Ok(false) => {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    };

    match run_with_timeout(pre_condition_future, pre_condition_timeout).await {
        Ok(true) => {
            let task_future = (task.func)(ctx);
            if task.repeatable {
                task_future
                    .await
                    .map_err(|e| {
                        eprintln!("Failed task {}, {e}", task.name);
                        e
                    })
                    .is_ok()
            } else {
                match run_with_timeout(task_future, task_timeout).await {
                    Ok(_) => true,
                    Err(e) => {
                        println!(
                            "Task function failed for node {:?}: {:?}",
                            &graph[node].name, e
                        );
                        false
                    }
                }
            }
        }
        Ok(false) => {
            println!("Precondition not met for node {:?}", &graph[node].name);
            false
        }
        Err(e) => {
            println!(
                "Precondition check timed out or failed for node {:?}: {:?}",
                &graph[node].name, e
            );
            false
        }
    }
}
#[tokio::main]
async fn main() {
    dotenv::dotenv().unwrap();
    let mut task_graph: Graph<Task, TaskEdge> = DiGraph::new();
    let browser_create = task_graph.add_node(Task::new_async(
        "Create browser",
        browser::open_browser_function,
        browser::open_browser_precondition,
        false,
    ));
    let navigate_wa = task_graph.add_node(Task::new_async(
        "Navigate to WA",
        browser::navigate_to_webadvisor_function,
        browser::navigate_to_webadvisor_precondition,
        false,
    ));

    let ms_email = task_graph.add_node(Task::new_async(
        "Login ms email",
        microsoft::login_microsoft_email_function,
        microsoft::login_microsoft_email_precondition,
        false,
    ));
    let ms_password = task_graph.add_node(Task::new_async(
        "Login ms password",
        microsoft::login_microsoft_password_function,
        microsoft::login_microsoft_password_precondition,
        false,
    ));
    let ms_otp = task_graph.add_node(Task::new_async(
        "Login ms otp",
        microsoft::login_microsoft_otp_function,
        microsoft::login_microsoft_otp_precondition,
        false,
    ));
    let wa_navigate = task_graph.add_node(Task::new_async(
        "Navigate WA",
        wa::wa_navigate_semester_function,
        wa::wa_navigate_semester_precondition,
        false,
    ));
    let wa_button = task_graph.add_node(Task::new_async(
        "Button WA",
        wa::wa_register_function,
        wa::wa_register_precondition,
        true,
    ));

    task_graph.add_edge(browser_create, navigate_wa, TaskEdge::Outgoing);
    task_graph.add_edge(navigate_wa, ms_email, TaskEdge::Outgoing);
    task_graph.add_edge(ms_email, ms_password, TaskEdge::Outgoing);
    task_graph.add_edge(ms_password, ms_otp, TaskEdge::Outgoing);
    task_graph.add_edge(ms_otp, wa_navigate, TaskEdge::Outgoing);
    task_graph.add_edge(wa_navigate, wa_button, TaskEdge::Outgoing);
    ensure_bidirectional_edges(&mut task_graph);
    let ctx = Context::default();
    loop {
        let mut stack: VecDeque<NodeIndex> = VecDeque::from(vec![browser_create]);
        while let Some(node) = stack.pop_front() {
            println!("{:?}", node);
            match execute_task(&task_graph, ctx.clone(), node).await {
                true => {}
                false => {
                    break;
                }
            };
            match run_with_timeout(
                {
                    let task_graph = &task_graph;
                    let ctx = ctx.clone();
                    async move {
                        loop {
                            for edge in task_graph.edges(node) {
                                if *edge.weight() != TaskEdge::Outgoing {
                                    continue;
                                }
                                let node = edge.target();
                                let task = &task_graph[node];
                                if (task.pre_condition)(ctx.clone()).await.ok().is_some() {
                                    return Ok(node);
                                } else {
                                    println!("Failed {}", &task_graph[node].name);
                                }
                            }
                            sleep(Duration::from_millis(10)).await;
                        }
                    }
                },
                Duration::from_secs(20),
            )
            .await
            {
                Ok(node) => {
                    println!("Success!");
                    stack.push_back(node);
                }
                Err(e) => {
                    eprintln!("Timed out! {e}");
                    break;
                }
            }
        }
    }
}
