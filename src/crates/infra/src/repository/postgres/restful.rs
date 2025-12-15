use sea_orm::*;
use std::marker::PhantomData;
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RestfulError {
    #[error("{0}")]
    DbErr(#[from] sea_orm::DbErr),
    #[error("{0}")]
    PrimaryKeyNotFound(String),
    //#[error("{0}")]
    //Unknown(String),
}

pub struct RestfulRepository<T> {
    _marker: PhantomData<T>,
    db: DatabaseConnection,
}

impl<T> RestfulRepository<T>
where
    T: ActiveModelTrait + ActiveModelBehavior + Send + 'static + Sync,
    <T::Entity as EntityTrait>::Model: IntoActiveModel<T> + Sync,
{
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            _marker: PhantomData,
            db,
        }
    }
    pub async fn create(
        &self,
        model: <T::Entity as EntityTrait>::Model,
    ) -> Result<(), RestfulError> {
        let mut active_model: T = model.into_active_model();
        for key in <T::Entity as EntityTrait>::PrimaryKey::iter() {
            let col = key.into_column();
            active_model.not_set(col);
        }
        active_model.insert(&self.db).await?;
        Ok(())
    }

    pub async fn delete_by_id(&self, pk: i64) -> Result<(), RestfulError> {
        let instance = <T::Entity as EntityTrait>::find_by_id(Self::exchange_primary_key(pk))
            .one(&self.db)
            .await?;

        if let Some(instance) = instance {
            instance.delete(&self.db).await?;
            Ok(())
        } else {
            Err(RestfulError::PrimaryKeyNotFound(format!(
                "Instance with id {} not found",
                pk
            )))
        }
    }

    fn set_model_primary_key(active_model: &mut T, value: i64) {
        if let Some(key) = <T::Entity as EntityTrait>::PrimaryKey::iter().next() {
            let col = key.into_column();
            active_model.set(col, sea_orm::Value::BigInt(Some(value as i64)));
        }
    }

    pub async fn update(
        &self,
        pk: i64,
        model: <T::Entity as EntityTrait>::Model,
    ) -> Result<(), RestfulError> {
        self.check_instance_exists(pk).await?;
        let active_model: T = model.into_active_model();
        let mut active_model = active_model.reset_all();
        Self::set_model_primary_key(&mut active_model, pk);
        active_model.update(&self.db).await?;
        Ok(())
    }

    async fn check_instance_exists(
        &self,
        pk: i64,
    ) -> Result<<T::Entity as EntityTrait>::Model, RestfulError> {
        let instance = <T::Entity as EntityTrait>::find_by_id(Self::exchange_primary_key(pk))
            .one(&self.db)
            .await?;

        instance.ok_or_else(|| {
            RestfulError::PrimaryKeyNotFound(format!("Instance with id {} not found", pk))
        })
    }

    pub async fn list(
        &self,
        page_size: i64,
        page_num: i64,
    ) -> Result<Vec<<T::Entity as EntityTrait>::Model>, RestfulError> {
        let results = if page_size > 0 {
            T::Entity::find()
                .paginate(&self.db, page_size as u64)
                .fetch_page(page_num as u64)
                .await?
        } else {
            T::Entity::find().all(&self.db).await?
        };
        Ok(results)
    }

    pub async fn get_by_id(
        &self,
        pk: i64,
    ) -> Result<<T::Entity as EntityTrait>::Model, RestfulError> {
        let instance = <T::Entity as EntityTrait>::find_by_id(Self::exchange_primary_key(pk))
            .one(&self.db)
            .await?;

        // 解包 instance，确保它存在
        let instance = instance.ok_or_else(|| {
            RestfulError::PrimaryKeyNotFound(format!("Instance with id {} not found", pk))
        })?;

        Ok(instance)
    }

    #[inline]
    fn exchange_primary_key(
        id: i64,
    ) -> <<T::Entity as EntityTrait>::PrimaryKey as PrimaryKeyTrait>::ValueType {
        <<T::Entity as EntityTrait>::PrimaryKey as PrimaryKeyTrait>::ValueType::try_from_u64(id as u64)
            .unwrap()
    }
}
