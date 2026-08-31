// ============================================================================
// ID 值类型 / ID value types
// ============================================================================

/// 甘特领域 ID / Gantt domain id
pub trait GanttId:
    Clone + std::fmt::Debug + Eq + std::hash::Hash + Ord + std::fmt::Display + Send + Sync + 'static
{
    /// ID 是否为空 / Whether the id is empty
    fn is_empty(&self) -> bool {
        self.to_string().is_empty()
    }
}

impl<I> GanttId for I where
    I: Clone
        + std::fmt::Debug
        + Eq
        + std::hash::Hash
        + Ord
        + std::fmt::Display
        + Send
        + Sync
        + 'static
{
}

/// 任务 ID trait / Task id trait
pub trait TaskIdTrait: GanttId {}

/// 执行者 ID trait / Executor id trait
pub trait ExecutorIdTrait: GanttId {}

/// 任务步骤 ID trait / Task step id trait
pub trait TaskStepIdTrait: GanttId {}

/// 任务计划 ID trait / Task plan id trait
pub trait TaskPlanIdTrait: GanttId {}

/// 资源 ID trait / Resource id trait
pub trait ResourceIdTrait: GanttId {}

/// 生产动作 ID trait / Production action id trait
pub trait ProductionActionIdTrait: GanttId {}

/// 生产物料 ID trait / Production material id trait
pub trait ProductionMaterialIdTrait: GanttId {}

macro_rules! string_id_type {
    ($name:ident, $doc_zh:literal, $doc_en:literal) => {
        #[doc = concat!($doc_zh, " / ", $doc_en)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(String);

        impl $name {
            /// 创建 ID / Create id
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            /// 字符串视图 / String view
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// 转为字符串 / Convert into string
            pub fn into_string(self) -> String {
                self.0
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_string())
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl std::ops::Deref for $name {
            type Target = str;

            fn deref(&self) -> &Self::Target {
                self.as_str()
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl std::borrow::Borrow<str> for $name {
            fn borrow(&self) -> &str {
                self.as_str()
            }
        }

        impl PartialEq<&str> for $name {
            fn eq(&self, other: &&str) -> bool {
                self.as_str() == *other
            }
        }

        impl PartialEq<str> for $name {
            fn eq(&self, other: &str) -> bool {
                self.as_str() == other
            }
        }

        impl PartialEq<String> for $name {
            fn eq(&self, other: &String) -> bool {
                self.as_str() == other
            }
        }

        impl PartialEq<$name> for str {
            fn eq(&self, other: &$name) -> bool {
                self == other.as_str()
            }
        }

        impl PartialEq<$name> for String {
            fn eq(&self, other: &$name) -> bool {
                self.as_str() == other.as_str()
            }
        }
    };
}

string_id_type!(TaskId, "任务 ID", "Task id");
string_id_type!(ExecutorId, "执行者 ID", "Executor id");
string_id_type!(TaskStepId, "任务步骤 ID", "Task step id");
string_id_type!(TaskPlanId, "任务计划 ID", "Task plan id");
string_id_type!(ResourceId, "资源 ID", "Resource id");
string_id_type!(ProductionActionId, "生产动作 ID", "Production action id");
string_id_type!(
    ProductionMaterialId,
    "生产物料 ID",
    "Production material id"
);

impl TaskIdTrait for TaskId {}
impl ExecutorIdTrait for ExecutorId {}
impl TaskStepIdTrait for TaskStepId {}
impl TaskPlanIdTrait for TaskPlanId {}
impl ResourceIdTrait for ResourceId {}
impl ProductionActionIdTrait for ProductionActionId {}
impl ProductionMaterialIdTrait for ProductionMaterialId {}

// 迁移期兼容纯字符串实现；新领域模型应使用独立 newtype。
// Preserve legacy string implementations during migration; new domain models should use newtypes.
impl TaskIdTrait for String {}
impl ExecutorIdTrait for String {}
impl TaskStepIdTrait for String {}
impl TaskPlanIdTrait for String {}
impl ResourceIdTrait for String {}
impl ProductionActionIdTrait for String {}
impl ProductionMaterialIdTrait for String {}

/// 创建任务 ID / Create task id
pub fn task_id(value: impl Into<String>) -> TaskId {
    TaskId::new(value)
}

/// 创建执行者 ID / Create executor id
pub fn executor_id(value: impl Into<String>) -> ExecutorId {
    ExecutorId::new(value)
}

/// 创建任务步骤 ID / Create task step id
pub fn task_step_id(value: impl Into<String>) -> TaskStepId {
    TaskStepId::new(value)
}

/// 创建任务计划 ID / Create task plan id
pub fn task_plan_id(value: impl Into<String>) -> TaskPlanId {
    TaskPlanId::new(value)
}

/// 创建资源 ID / Create resource id
pub fn resource_id(value: impl Into<String>) -> ResourceId {
    ResourceId::new(value)
}

/// 创建生产动作 ID / Create production action id
pub fn production_action_id(value: impl Into<String>) -> ProductionActionId {
    ProductionActionId::new(value)
}

/// 创建生产物料 ID / Create production material id
pub fn production_material_id(value: impl Into<String>) -> ProductionMaterialId {
    ProductionMaterialId::new(value)
}
