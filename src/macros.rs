#[macro_export]
#[doc(hidden)]
macro_rules! __eager_load_belongs_to {
    ($relation_name:ident, $fk:ident, $lk:ident, $related_model:path, $db:expr, $models:expr) => {
        let ids: Vec<_> = $models.iter()
            .map(|m| $crate::serde_json::to_value(&m.$fk).unwrap_or($crate::serde_json::Value::Null))
            .filter(|v| !v.is_null())
            .collect();
        let related = <$related_model>::query($db).where_in(stringify!($lk), ids).get::<$related_model>().await?;
        for m in &mut *$models {
            m.$relation_name = related.iter().find(|r| {
                let r_val = $crate::serde_json::to_value(&r.$lk).unwrap_or($crate::serde_json::Value::Null);
                let m_val = $crate::serde_json::to_value(&m.$fk).unwrap_or($crate::serde_json::Value::Null);
                r_val == m_val && !r_val.is_null()
            }).cloned();
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __eager_load_has_many {
    ($relation_name:ident, $fk:ident, $lk:ident, $related_model:path, $db:expr, $models:expr) => {
        let ids: Vec<_> = $models.iter()
            .map(|m| $crate::serde_json::to_value(&m.$lk).unwrap_or($crate::serde_json::Value::Null))
            .filter(|v| !v.is_null())
            .collect();
        let related = <$related_model>::query($db).where_in(stringify!($fk), ids).get::<$related_model>().await?;
        for m in &mut *$models {
            let matched: Vec<$related_model> = related.iter().filter(|r| {
                let r_val = $crate::serde_json::to_value(&r.$fk).unwrap_or($crate::serde_json::Value::Null);
                let m_val = $crate::serde_json::to_value(&m.$lk).unwrap_or($crate::serde_json::Value::Null);
                r_val == m_val && !r_val.is_null()
            }).cloned().collect();
            m.$relation_name = Some(matched);
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __eager_load_dispatcher {
    (belongs_to, $relation_name:ident, $fk:ident, $lk:ident, $related_model:path, $db:expr, $models:expr) => {
        $crate::__eager_load_belongs_to!($relation_name, $fk, $lk, $related_model, $db, $models);
    };
    (has_many, $relation_name:ident, $fk:ident, $lk:ident, $related_model:path, $db:expr, $models:expr) => {
        $crate::__eager_load_has_many!($relation_name, $fk, $lk, $related_model, $db, $models);
    };
}

#[macro_export]
macro_rules! model {
    (
        table: $table_name:expr,
        $(timestamps: $ts:expr,)?
        $(soft_deletes: $sd:expr,)?
        $(fillable: [ $($fill:ident),* ],)?
        $(guarded: [ $($guard:ident),* ],)?
        $(scopes: {
            $( $scope_name:ident ( $($arg_name:ident : $arg_type:ty),* ) => $scope_body:expr ),* $(,)?
        },)?
        $(global_scopes: {
            $( $gs_name:ident => $gs_body:expr ),* $(,)?
        },)?
        $(relations: {
            $( $relation_name:ident ( $rel_type:ident, foreign_key: $fk:ident, local_key: $lk:ident ) => $related_model:path ),* $(,)?
        },)?
        Model {
            $($(#[$field_meta:meta])* $field_vis:vis $field_name:ident : $field_type:ty),* $(,)?
        }
    ) => {
        use $crate::serde as _serde;

        #[derive(Clone, Debug, PartialEq, _serde::Serialize, _serde::Deserialize)]
        #[serde(crate = "_serde")]
        pub struct Model {
            $($(#[$field_meta])* $field_vis $field_name : $field_type,)*
        }

        pub type Entity = Model;

        $(
            pub trait ModelScopes<'a> {
                $(
                    fn $scope_name(self, $($arg_name: $arg_type),*) -> Self;
                )*
            }

            impl<'a> ModelScopes<'a> for $crate::database::QueryBuilder<'a> {
                $(
                    fn $scope_name(self, $($arg_name: $arg_type),*) -> Self {
                        let f: fn($crate::database::QueryBuilder<'a>, $($arg_type),*) -> $crate::database::QueryBuilder<'a> = $scope_body;
                        f(self, $($arg_name),*)
                    }
                )*
            }
        )?

        pub struct ModelQuery<'a> {
            builder: $crate::database::QueryBuilder<'a>,
            relations: Vec<String>,
        }

        impl<'a> ModelQuery<'a> {
            pub fn new(db: &'a $crate::sql::AnyPool) -> Self {
                Self {
                    builder: Model::query(db),
                    relations: Vec::new(),
                }
            }

            pub fn with(mut self, relations: &[&str]) -> Self {
                self.relations.extend(relations.iter().map(|r| r.to_string()));
                self
            }

            pub fn where_(mut self, column: &str, value: impl $crate::serde::Serialize) -> Self {
                self.builder = self.builder.where_(column, value);
                self
            }

            pub fn where_op(mut self, column: &str, operator: &str, value: impl $crate::serde::Serialize) -> Self {
                self.builder = self.builder.where_op(column, operator, value);
                self
            }

            pub fn where_raw(mut self, sql: &str, binds: Vec<$crate::serde_json::Value>) -> Self {
                self.builder = self.builder.where_raw(sql, binds);
                self
            }

            pub fn order_by(mut self, column: &str, direction: &str) -> Self {
                self.builder = self.builder.order_by(column, direction);
                self
            }

            pub fn limit(mut self, limit: usize) -> Self {
                self.builder = self.builder.limit(limit);
                self
            }

            pub async fn get(self) -> Result<Vec<Model>, $crate::sql::Error> {
                let db = self.builder.pool();
                let mut models = self.builder.get::<Model>().await?;

                // Apply eager loading for relations if defined
                $(
                    for rel in &self.relations {
                        match rel.as_str() {
                            $(
                                stringify!($relation_name) => {
                                    $crate::__eager_load_dispatcher!($rel_type, $relation_name, $fk, $lk, $related_model, db, &mut models);
                                }
                            )*
                            _ => {}
                        }
                    }
                )?

                Ok(models)
            }

            pub async fn first(self) -> Result<Option<Model>, $crate::sql::Error> {
                let mut models = self.get().await?;
                if models.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(models.remove(0)))
                }
            }
        }

        impl Model {
            /// Mulai memuat relasi secara eager (Model::with([...]))
            pub fn with<'a>(db: &'a $crate::sql::AnyPool, relations: &[&str]) -> ModelQuery<'a> {
                ModelQuery::new(db).with(relations)
            }

            /// Mulai Query Builder baru untuk tabel model ini (Model::query())
            pub fn query<'a>(db: &'a $crate::sql::AnyPool) -> $crate::database::QueryBuilder<'a> {
                let mut q = $crate::database::DB::table(db, $table_name);
                let mut has_soft_deletes = false;
                $(
                    if $sd {
                        has_soft_deletes = true;
                    }
                )?
                if has_soft_deletes {
                    q = q.where_raw("`deleted_at` IS NULL", vec![]);
                }

                // Terapkan scope global khusus
                $(
                    $(
                        q = {
                            let f: fn($crate::database::QueryBuilder<'a>) -> $crate::database::QueryBuilder<'a> = $gs_body;
                            f(q)
                        };
                    )*
                )?

                q
            }

            /// Mulai query builder tanpa menerapkan scope global apa pun (termasuk soft deletes)
            pub fn query_without_global_scopes<'a>(db: &'a $crate::sql::AnyPool) -> $crate::database::QueryBuilder<'a> {
                $crate::database::DB::table(db, $table_name)
            }

            /// Ambil semua catatan dari model ini (Model::all())
            pub async fn all(db: &$crate::sql::AnyPool) -> Result<Vec<Self>, $crate::sql::Error> {
                Self::query(db).get::<Self>().await
            }

            /// Ambil catatan pertama dari model ini (Model::first())
            pub async fn first(db: &$crate::sql::AnyPool) -> Result<Option<Self>, $crate::sql::Error> {
                Self::query(db).first::<Self>().await
            }

            /// Cari catatan berdasarkan ID numeriknya (Model::find($id))
            pub async fn find(db: &$crate::sql::AnyPool, id: i32) -> Result<Option<Self>, $crate::sql::Error> {
                Self::query(db).where_("id", id).first::<Self>().await
            }

            /// Mengambil total jumlah data dari model ini (Model::count())
            pub async fn count(db: &$crate::sql::AnyPool) -> Result<i64, $crate::sql::Error> {
                Self::query(db).count().await
            }

            /// Menghapus data berdasarkan ID (Model::destroy($id))
            pub async fn destroy(db: &$crate::sql::AnyPool, id: i32) -> Result<u64, $crate::sql::Error> {
                let mut has_soft_deletes = false;
                $(
                    if $sd {
                        has_soft_deletes = true;
                    }
                )?
                if has_soft_deletes {
                    $crate::database::DB::table(db, $table_name)
                        .where_("id", id)
                        .update($crate::serde_json::json!({
                            "deleted_at": $crate::chrono::Utc::now().naive_utc()
                        }))
                        .await
                } else {
                    $crate::database::DB::table(db, $table_name).where_("id", id).delete().await
                }
            }

            /// Mulai query builder dengan menyertakan data yang telah dihapus secara lunak (Model::withTrashed())
            pub fn query_with_trashed<'a>(db: &'a $crate::sql::AnyPool) -> $crate::database::QueryBuilder<'a> {
                $crate::database::DB::table(db, $table_name)
            }

            /// Mulai query builder hanya untuk data yang telah dihapus secara lunak (Model::onlyTrashed())
            pub fn query_only_trashed<'a>(db: &'a $crate::sql::AnyPool) -> $crate::database::QueryBuilder<'a> {
                $crate::database::DB::table(db, $table_name).where_raw("`deleted_at` IS NOT NULL", vec![])
            }

            /// Memulihkan data yang telah dihapus secara lunak ($model->restore())
            pub async fn restore(db: &$crate::sql::AnyPool, id: i32) -> Result<u64, $crate::sql::Error> {
                $crate::database::DB::table(db, $table_name)
                    .where_("id", id)
                    .update($crate::serde_json::json!({
                        "deleted_at": $crate::serde_json::Value::Null
                    }))
                    .await
            }

            /// Menghapus data secara permanen ($model->force_destroy(id))
            pub async fn force_destroy(db: &$crate::sql::AnyPool, id: i32) -> Result<u64, $crate::sql::Error> {
                $crate::database::DB::table(db, $table_name).where_("id", id).delete().await
            }

            $(
                $(
                    pub fn $scope_name<'a>(db: &'a $crate::sql::AnyPool, $($arg_name: $arg_type),*) -> $crate::database::QueryBuilder<'a> {
                        use self::ModelScopes;
                        Self::query(db).$scope_name($($arg_name),*)
                    }
                )*
            )?

            pub async fn create(db: &$crate::sql::AnyPool, mut data: $crate::serde_json::Value) -> Result<Self, $crate::sql::Error> {
                let mut data_to_insert = data.clone();

                // Terapkan penyaringan Fillable jika didefinisikan
                $(
                    let fillable_keys: Vec<&str> = vec![ $( stringify!($fill) ),* ];
                    if let Some(obj) = data.as_object() {
                        let mut filtered_obj = $crate::serde_json::Map::new();
                        for key in fillable_keys {
                            if let Some(val) = obj.get(key) {
                                filtered_obj.insert(key.to_string(), val.clone());
                            }
                        }
                        data_to_insert = $crate::serde_json::Value::Object(filtered_obj);
                    }
                )?

                // Terapkan penyaringan Guarded jika didefinisikan
                $(
                    let guarded_keys: Vec<&str> = vec![ $( stringify!($guard) ),* ];
                    if let Some(obj) = data_to_insert.as_object_mut() {
                        for key in guarded_keys {
                            obj.remove(key);
                        }
                    }
                )?

                let res = $crate::database::DB::table(db, $table_name).insert_get_id(data_to_insert.clone()).await;
                match res {
                    Ok(id) => {
                        if let Some(obj) = data.as_object_mut() {
                            if !obj.contains_key("id") {
                                obj.insert("id".to_string(), $crate::serde_json::json!(id));
                            }
                        }
                    }
                    Err(e) => {
                        println!("insert_get_id failed: {:?}", e);
                        $crate::database::DB::table(db, $table_name).insert(data_to_insert.clone()).await?;
                    }
                }
                let parsed = $crate::serde_json::from_value::<Self>(data)
                    .map_err(|e| $crate::sql::Error::Protocol(format!("Deserialization error: {}", e)))?;
                Ok(parsed)
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
            async fn run<'a>(&'a self, $db: &'a $crate::sql::AnyPool) -> Result<(), $crate::sql::Error> $body
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
