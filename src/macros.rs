#[macro_export]
#[doc(hidden)]
macro_rules! model_timestamps_fields {
    (true) => {
        pub created_at: Option<DateTime>,
        pub updated_at: Option<DateTime>,
    };
    (false) => {};
}

#[macro_export]
#[doc(hidden)]
macro_rules! model_behavior_impl {
    (true) => {
        #[$crate::async_trait]
        impl $crate::sea_orm::entity::prelude::ActiveModelBehavior for ActiveModel {
            async fn before_save<C>(mut self, _db: &C, insert: bool) -> Result<Self, $crate::sea_orm::DbErr>
            where
                C: $crate::sea_orm::ConnectionTrait,
            {
                let now = $crate::chrono::Local::now().naive_local();
                if insert {
                    self.created_at = $crate::sea_orm::ActiveValue::Set(Some(now));
                }
                self.updated_at = $crate::sea_orm::ActiveValue::Set(Some(now));
                Ok(self)
            }
        }
    };
    (false) => {
        impl $crate::sea_orm::entity::prelude::ActiveModelBehavior for ActiveModel {}
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! model_impl {
    // Branch A: timestamps: true
    (
        table: $table_name:expr,
        timestamps: true,
        fillable: [ $($fill:ident),* ],
        guarded: [ $($guard:ident),* ],
        Model {
            $($(#[$field_meta:meta])* $field_vis:vis $field_name:ident : $field_type:ty),* $(,)?
        }
    ) => {
        #[derive(Clone, Debug, PartialEq, $crate::sea_orm::entity::prelude::DeriveEntityModel, $crate::serde::Serialize, $crate::serde::Deserialize)]
        #[sea_orm(table_name = $table_name)]
        pub struct Model {
            $($(#[$field_meta])* $field_vis $field_name : $field_type,)*
            pub created_at: Option<DateTime>,
            pub updated_at: Option<DateTime>,
        }

        #[derive(Copy, Clone, Debug, $crate::sea_orm::entity::prelude::EnumIter, $crate::sea_orm::entity::prelude::DeriveRelation)]
        pub enum Relation {}

        $crate::model_behavior_impl!(true);

        impl Entity {
            pub async fn create<C>(db: &C, data: $crate::serde_json::Value) -> Result<Model, $crate::sea_orm::DbErr>
            where
                C: $crate::sea_orm::ConnectionTrait,
            {
                let active = ActiveModel::fill(&data)?;
                <ActiveModel as $crate::sea_orm::ActiveModelTrait>::insert(active, db).await
            }
        }

        impl Model {
            pub async fn create<C>(db: &C, data: $crate::serde_json::Value) -> Result<Self, $crate::sea_orm::DbErr>
            where
                C: $crate::sea_orm::ConnectionTrait,
            {
                let active = ActiveModel::fill(&data)?;
                <ActiveModel as $crate::sea_orm::ActiveModelTrait>::insert(active, db).await
            }
        }

        impl ActiveModel {
            pub fn fill(json: &$crate::serde_json::Value) -> Result<Self, $crate::sea_orm::DbErr> {
                let mut active: Self = ::std::default::Default::default();
                if let Some(obj) = json.as_object() {
                    let fillable: Vec<&str> = vec![ $( stringify!($fill) ),* ];
                    let guarded: Vec<&str> = vec![ $( stringify!($guard) ),* ];
                    
                    $(
                        let key = stringify!($field_name);
                        let is_allowed = if !fillable.is_empty() {
                            fillable.contains(&key)
                        } else if !guarded.is_empty() {
                            !guarded.contains(&key)
                        } else {
                            true
                        };
                        
                        if is_allowed {
                            if let Some(val) = obj.get(key) {
                                let parsed: $field_type = $crate::serde_json::from_value(val.clone())
                                    .map_err(|e| $crate::sea_orm::DbErr::Custom(format!("Json Error: {}", e)))?;
                                active.$field_name = $crate::sea_orm::ActiveValue::Set(parsed);
                            }
                        }
                    )*
                }
                Ok(active)
            }
        }
    };

    // Branch B: timestamps: false
    (
        table: $table_name:expr,
        timestamps: false,
        fillable: [ $($fill:ident),* ],
        guarded: [ $($guard:ident),* ],
        Model {
            $($(#[$field_meta:meta])* $field_vis:vis $field_name:ident : $field_type:ty),* $(,)?
        }
    ) => {
        #[derive(Clone, Debug, PartialEq, $crate::sea_orm::entity::prelude::DeriveEntityModel, $crate::serde::Serialize, $crate::serde::Deserialize)]
        #[sea_orm(table_name = $table_name)]
        pub struct Model {
            $($(#[$field_meta])* $field_vis $field_name : $field_type,)*
        }

        #[derive(Copy, Clone, Debug, $crate::sea_orm::entity::prelude::EnumIter, $crate::sea_orm::entity::prelude::DeriveRelation)]
        pub enum Relation {}

        $crate::model_behavior_impl!(false);

        impl Entity {
            pub async fn create<C>(db: &C, data: $crate::serde_json::Value) -> Result<Model, $crate::sea_orm::DbErr>
            where
                C: $crate::sea_orm::ConnectionTrait,
            {
                let active = ActiveModel::fill(&data)?;
                <ActiveModel as $crate::sea_orm::ActiveModelTrait>::insert(active, db).await
            }
        }

        impl Model {
            pub async fn create<C>(db: &C, data: $crate::serde_json::Value) -> Result<Self, $crate::sea_orm::DbErr>
            where
                C: $crate::sea_orm::ConnectionTrait,
            {
                let active = ActiveModel::fill(&data)?;
                <ActiveModel as $crate::sea_orm::ActiveModelTrait>::insert(active, db).await
            }
        }

        impl ActiveModel {
            pub fn fill(json: &$crate::serde_json::Value) -> Result<Self, $crate::sea_orm::DbErr> {
                let mut active: Self = ::std::default::Default::default();
                if let Some(obj) = json.as_object() {
                    let fillable: Vec<&str> = vec![ $( stringify!($fill) ),* ];
                    let guarded: Vec<&str> = vec![ $( stringify!($guard) ),* ];
                    
                    $(
                        let key = stringify!($field_name);
                        let is_allowed = if !fillable.is_empty() {
                            fillable.contains(&key)
                        } else if !guarded.is_empty() {
                            !guarded.contains(&key)
                        } else {
                            true
                        };
                        
                        if is_allowed {
                            if let Some(val) = obj.get(key) {
                                let parsed: $field_type = $crate::serde_json::from_value(val.clone())
                                    .map_err(|e| $crate::sea_orm::DbErr::Custom(format!("Json Error: {}", e)))?;
                                active.$field_name = $crate::sea_orm::ActiveValue::Set(parsed);
                            }
                        }
                    )*
                }
                Ok(active)
            }
        }
    };
}

#[macro_export]
macro_rules! model {
    // 1. All options provided with true
    (
        table: $table_name:expr,
        timestamps: true,
        fillable: [ $($fill:ident),* ],
        guarded: [ $($guard:ident),* ],
        Model {
            $($(#[$field_meta:meta])* $field_vis:vis $field_name:ident : $field_type:ty),* $(,)?
        }
    ) => {
        $crate::model_impl! {
            table: $table_name,
            timestamps: true,
            fillable: [ $($fill),* ],
            guarded: [ $($guard),* ],
            Model {
                $($(#[$field_meta])* $field_vis $field_name : $field_type,)*
            }
        }
    };

    // 2. All options provided with false
    (
        table: $table_name:expr,
        timestamps: false,
        fillable: [ $($fill:ident),* ],
        guarded: [ $($guard:ident),* ],
        Model {
            $($(#[$field_meta:meta])* $field_vis:vis $field_name:ident : $field_type:ty),* $(,)?
        }
    ) => {
        $crate::model_impl! {
            table: $table_name,
            timestamps: false,
            fillable: [ $($fill),* ],
            guarded: [ $($guard),* ],
            Model {
                $($(#[$field_meta])* $field_vis $field_name : $field_type,)*
            }
        }
    };

    // 3. Defaulting guarded (empty)
    (
        table: $table_name:expr,
        timestamps: $ts:ident,
        fillable: [ $($fill:ident),* ],
        Model {
            $($(#[$field_meta:meta])* $field_vis:vis $field_name:ident : $field_type:ty),* $(,)?
        }
    ) => {
        $crate::model! {
            table: $table_name,
            timestamps: $ts,
            fillable: [ $($fill),* ],
            guarded: [ ],
            Model {
                $($(#[$field_meta])* $field_vis $field_name : $field_type,)*
            }
        }
    };

    // 4. Defaulting fillable (empty)
    (
        table: $table_name:expr,
        timestamps: $ts:ident,
        guarded: [ $($guard:ident),* ],
        Model {
            $($(#[$field_meta:meta])* $field_vis:vis $field_name:ident : $field_type:ty),* $(,)?
        }
    ) => {
        $crate::model! {
            table: $table_name,
            timestamps: $ts,
            fillable: [ ],
            guarded: [ $($guard),* ],
            Model {
                $($(#[$field_meta])* $field_vis $field_name : $field_type,)*
            }
        }
    };

    // 5. Defaulting both fillable and guarded (empty)
    (
        table: $table_name:expr,
        timestamps: $ts:ident,
        Model {
            $($(#[$field_meta:meta])* $field_vis:vis $field_name:ident : $field_type:ty),* $(,)?
        }
    ) => {
        $crate::model! {
            table: $table_name,
            timestamps: $ts,
            fillable: [ ],
            guarded: [ ],
            Model {
                $($(#[$field_meta])* $field_vis $field_name : $field_type,)*
            }
        }
    };

    // 6. Defaulting timestamps: true
    (
        table: $table_name:expr,
        fillable: [ $($fill:ident),* ],
        guarded: [ $($guard:ident),* ],
        Model {
            $($(#[$field_meta:meta])* $field_vis:vis $field_name:ident : $field_type:ty),* $(,)?
        }
    ) => {
        $crate::model! {
            table: $table_name,
            timestamps: true,
            fillable: [ $($fill),* ],
            guarded: [ $($guard),* ],
            Model {
                $($(#[$field_meta])* $field_vis $field_name : $field_type,)*
            }
        }
    };

    // 7. Defaulting timestamps: true, guarded: empty
    (
        table: $table_name:expr,
        fillable: [ $($fill:ident),* ],
        Model {
            $($(#[$field_meta:meta])* $field_vis:vis $field_name:ident : $field_type:ty),* $(,)?
        }
    ) => {
        $crate::model! {
            table: $table_name,
            timestamps: true,
            fillable: [ $($fill),* ],
            guarded: [ ],
            Model {
                $($(#[$field_meta])* $field_vis $field_name : $field_type,)*
            }
        }
    };

    // 8. Defaulting timestamps: true, fillable: empty
    (
        table: $table_name:expr,
        guarded: [ $($guard:ident),* ],
        Model {
            $($(#[$field_meta:meta])* $field_vis:vis $field_name:ident : $field_type:ty),* $(,)?
        }
    ) => {
        $crate::model! {
            table: $table_name,
            timestamps: true,
            fillable: [ ],
            guarded: [ $($guard),* ],
            Model {
                $($(#[$field_meta])* $field_vis $field_name : $field_type,)*
            }
        }
    };

    // 9. Defaulting all: timestamps: true, fillable: empty, guarded: empty
    (
        table: $table_name:expr,
        Model {
            $($(#[$field_meta:meta])* $field_vis:vis $field_name:ident : $field_type:ty),* $(,)?
        }
    ) => {
        $crate::model! {
            table: $table_name,
            timestamps: true,
            fillable: [ ],
            guarded: [ ],
            Model {
                $($(#[$field_meta])* $field_vis $field_name : $field_type,)*
            }
        }
    };
}

#[macro_export]
macro_rules! seeder {
    (
        $name:ident,
        run($db:ident) $body:block
    ) => {
        pub struct $name;

        #[$crate::async_trait]
        impl $crate::seeder::SeederTrait for $name {
            async fn run(&self, $db: &$crate::sea_orm::DatabaseConnection) -> Result<(), $crate::sea_orm::DbErr> {
                $body
            }
        }
    };

    (
        run($db:ident) $body:block
    ) => {
        $crate::seeder! {
            DatabaseSeeder,
            run($db) $body
        }
    };
}
