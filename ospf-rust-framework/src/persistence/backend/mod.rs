//! 持久化后端入口
//! Persistence backend entry points

#[cfg(feature = "persistence-cornucopia")]
pub mod cornucopia;
#[cfg(feature = "persistence-diesel")]
pub mod diesel;
#[cfg(feature = "persistence-mongodb")]
pub mod mongodb;
#[cfg(feature = "persistence-rbatis")]
pub mod rbatis;
#[cfg(feature = "persistence-redis")]
pub mod redis;
#[cfg(feature = "persistence-sea-orm")]
pub mod sea_orm;
#[cfg(feature = "persistence-sqlx")]
pub mod sqlx;
#[cfg(feature = "persistence-toasty")]
pub mod toasty;

/// 后端 feature 能力快照。
/// Backend feature capability snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PersistenceBackendFeatures {
    pub sqlx: bool,
    pub sqlx_postgres: bool,
    pub sqlx_mysql: bool,
    pub sqlx_sqlite: bool,
    pub diesel: bool,
    pub diesel_postgres: bool,
    pub diesel_mysql: bool,
    pub diesel_sqlite: bool,
    pub sea_orm: bool,
    pub toasty: bool,
    pub rbatis: bool,
    pub cornucopia: bool,
    pub mongodb: bool,
    pub redis: bool,
}

impl PersistenceBackendFeatures {
    /// 从编译期 feature 创建快照。
    /// Create a snapshot from compile-time features.
    pub const fn current() -> Self {
        Self {
            sqlx: cfg!(feature = "persistence-sqlx"),
            sqlx_postgres: cfg!(feature = "persistence-sqlx-postgres"),
            sqlx_mysql: cfg!(feature = "persistence-sqlx-mysql"),
            sqlx_sqlite: cfg!(feature = "persistence-sqlx-sqlite"),
            diesel: cfg!(feature = "persistence-diesel"),
            diesel_postgres: cfg!(feature = "persistence-diesel-postgres"),
            diesel_mysql: cfg!(feature = "persistence-diesel-mysql"),
            diesel_sqlite: cfg!(feature = "persistence-diesel-sqlite"),
            sea_orm: cfg!(feature = "persistence-sea-orm"),
            toasty: cfg!(feature = "persistence-toasty"),
            rbatis: cfg!(feature = "persistence-rbatis"),
            cornucopia: cfg!(feature = "persistence-cornucopia"),
            mongodb: cfg!(feature = "persistence-mongodb"),
            redis: cfg!(feature = "persistence-redis"),
        }
    }

    /// 判断是否启用任意后端。
    /// Check whether any backend is enabled.
    pub const fn any_enabled(self) -> bool {
        self.sqlx
            || self.diesel
            || self.sea_orm
            || self.toasty
            || self.rbatis
            || self.cornucopia
            || self.mongodb
            || self.redis
    }
}
