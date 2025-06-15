use std::time::SystemTime;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::process::Child;
use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct TaskInfo {
    pub id: u32,
    pub command: String,
    pub start_time: SystemTime,
    pub status: TaskStatus,
}

#[derive(Debug, Clone)]
pub enum TaskStatus {
    Running,
    Completed(i32), // exit code
    Failed(String),
}

pub struct TaskManager {
    tasks: Arc<Mutex<HashMap<u32, TaskInfo>>>,
    children: Arc<Mutex<HashMap<u32, Child>>>,
    next_id: Arc<Mutex<u32>>,
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            children: Arc::new(Mutex::new(HashMap::new())),
            next_id: Arc::new(Mutex::new(1)),
        }
    }

    pub fn add_task(&self, command: String, child: Child) -> u32 {
        let mut next_id = self.next_id.lock().unwrap();
        let id = *next_id;
        *next_id += 1;

        let task_info = TaskInfo {
            id,
            command,
            start_time: SystemTime::now(),
            status: TaskStatus::Running,
        };

        self.tasks.lock().unwrap().insert(id, task_info);
        self.children.lock().unwrap().insert(id, child);
        id
    }

    pub fn get_task(&self, id: u32) -> Option<TaskInfo> {
        self.tasks.lock().unwrap().get(&id).cloned()
    }

    pub fn list_tasks(&self) -> Vec<TaskInfo> {
        self.tasks.lock().unwrap().values().cloned().collect()
    }

    pub async fn check_task_status(&self, id: u32) -> Option<TaskStatus> {
        let mut children = self.children.lock().unwrap();
        if let Some(mut child) = children.remove(&id) {
            match child.try_wait() {
                Ok(Some(status)) => {
                    let task_status = if status.success() {
                        TaskStatus::Completed(status.code().unwrap_or(0))
                    } else {
                        TaskStatus::Failed(format!("Exit code: {:?}", status.code()))
                    };
                    
                    if let Some(task) = self.tasks.lock().unwrap().get_mut(&id) {
                        task.status = task_status.clone();
                    }
                    Some(task_status)
                }
                Ok(None) => {
                    children.insert(id, child);
                    Some(TaskStatus::Running)
                }
                Err(e) => {
                    let task_status = TaskStatus::Failed(e.to_string());
                    if let Some(task) = self.tasks.lock().unwrap().get_mut(&id) {
                        task.status = task_status.clone();
                    }
                    Some(task_status)
                }
            }
        } else {
            self.tasks.lock().unwrap().get(&id).map(|t| t.status.clone())
        }
    }
}
