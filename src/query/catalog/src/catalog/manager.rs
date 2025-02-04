// Copyright 2021 Datafuse Labs
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use common_base::base::GlobalInstance;
use common_config::CatalogConfig;
use common_config::InnerConfig;
use common_exception::ErrorCode;
use common_exception::Result;
use common_meta_api::SchemaApi;
use common_meta_app::schema::CatalogId;
use common_meta_app::schema::CatalogInfo;
use common_meta_app::schema::CatalogMeta;
use common_meta_app::schema::CatalogNameIdent;
use common_meta_app::schema::CatalogOption;
use common_meta_app::schema::CatalogType;
use common_meta_app::schema::CreateCatalogReq;
use common_meta_app::schema::DropCatalogReq;
use common_meta_app::schema::GetCatalogReq;
use common_meta_app::schema::HiveCatalogOption;
use common_meta_app::schema::ListCatalogReq;
use common_meta_store::MetaStore;
use common_meta_store::MetaStoreProvider;

use super::Catalog;
use super::CatalogCreator;

pub const CATALOG_DEFAULT: &str = "default";

pub struct CatalogManager {
    pub meta: MetaStore,

    /// default_catalog is the DEFAULT catalog.
    pub default_catalog: Arc<dyn Catalog>,
    /// external catalogs is the external catalogs that configured in config.
    pub external_catalogs: HashMap<String, Arc<dyn Catalog>>,

    /// catalog_creators is the catalog creators that registered.
    pub catalog_creators: HashMap<CatalogType, Arc<dyn CatalogCreator>>,
}

impl CatalogManager {
    /// Fetch catalog manager from global instance.
    pub fn instance() -> Arc<CatalogManager> {
        GlobalInstance::get()
    }

    /// Init the catalog manager in global instance.
    #[async_backtrace::framed]
    pub async fn init(
        conf: &InnerConfig,
        default_catalog: Arc<dyn Catalog>,
        catalog_creators: Vec<(CatalogType, Arc<dyn CatalogCreator>)>,
    ) -> Result<()> {
        GlobalInstance::set(Self::try_create(conf, default_catalog, catalog_creators).await?);

        Ok(())
    }

    /// Try to create a catalog manager via Config.
    #[async_backtrace::framed]
    async fn try_create(
        conf: &InnerConfig,
        default_catalog: Arc<dyn Catalog>,
        catalog_creators: Vec<(CatalogType, Arc<dyn CatalogCreator>)>,
    ) -> Result<Arc<CatalogManager>> {
        let meta = {
            let provider = Arc::new(MetaStoreProvider::new(conf.meta.to_meta_grpc_client_conf()));

            provider.create_meta_store().await?
        };

        let tenant = conf.query.tenant_id.clone();
        let catalog_creators = HashMap::from_iter(catalog_creators.into_iter());

        // init external catalogs.
        let mut external_catalogs = HashMap::default();
        for (name, ctl_cfg) in conf.catalogs.iter() {
            let CatalogConfig::Hive(hive_ctl_cfg) = ctl_cfg;
            let creator = catalog_creators.get(&CatalogType::Hive).ok_or_else(|| {
                ErrorCode::BadArguments(format!("unknown catalog type: {:?}", CatalogType::Hive))
            })?;
            let ctl = creator.try_create(&CatalogInfo {
                id: CatalogId { catalog_id: 0 },
                name_ident: CatalogNameIdent {
                    tenant: tenant.clone(),
                    catalog_name: name.clone(),
                },
                meta: CatalogMeta {
                    catalog_option: CatalogOption::Hive(HiveCatalogOption {
                        address: hive_ctl_cfg.metastore_address.clone(),
                        storage_params: None,
                    }),
                    created_on: Utc::now(),
                },
            })?;
            external_catalogs.insert(name.clone(), ctl);
        }

        let catalog_manager = Self {
            meta,
            default_catalog,
            external_catalogs,
            catalog_creators,
        };

        Ok(Arc::new(catalog_manager))
    }

    /// Get default catalog from manager.
    ///
    /// There are some place that we don't have async context, so we provide
    /// `get_default_catalog` to allow users fetch default catalog without async.
    pub fn get_default_catalog(&self) -> Result<Arc<dyn Catalog>> {
        Ok(self.default_catalog.clone())
    }

    /// build_catalog builds a catalog from catalog info.
    pub fn build_catalog(&self, info: &CatalogInfo) -> Result<Arc<dyn Catalog>> {
        let typ = info.meta.catalog_option.catalog_type();

        if typ == CatalogType::Default {
            return Ok(self.default_catalog.clone());
        }

        let creator = self
            .catalog_creators
            .get(&typ)
            .ok_or_else(|| ErrorCode::BadArguments(format!("unknown catalog type: {:?}", typ)))?;

        creator.try_create(info)
    }

    /// Get a catalog from manager.
    ///
    /// # NOTES
    ///
    /// DEFAULT catalog is handled specially via `get_default_catalog`. Other catalogs
    /// will be fetched from metasrv.
    #[async_backtrace::framed]
    pub async fn get_catalog(&self, tenant: &str, catalog_name: &str) -> Result<Arc<dyn Catalog>> {
        if catalog_name == CATALOG_DEFAULT {
            return self.get_default_catalog();
        }

        if let Some(ctl) = self.external_catalogs.get(catalog_name) {
            return Ok(ctl.clone());
        }

        // Get catalog from metasrv.
        let info = self
            .meta
            .get_catalog(GetCatalogReq::new(tenant, catalog_name))
            .await?;

        self.build_catalog(&info)
    }

    /// Create a new catalog.
    ///
    /// # NOTES
    ///
    /// Trying to create default catalog will return an error.
    #[async_backtrace::framed]
    pub async fn create_catalog(&self, req: CreateCatalogReq) -> Result<()> {
        if req.catalog_name() == CATALOG_DEFAULT {
            return Err(ErrorCode::BadArguments(
                "default catalog cannot be created".to_string(),
            ));
        }

        if self.external_catalogs.get(req.catalog_name()).is_some() {
            return Err(ErrorCode::BadArguments(
                "catalog already exists that cannot be created".to_string(),
            ));
        }

        let _ = self.meta.create_catalog(req).await;

        Ok(())
    }

    /// Drop a catalog.
    ///
    /// # NOTES
    ///
    /// Trying to drop default catalog will return an error.
    #[async_backtrace::framed]
    pub async fn drop_catalog(&self, req: DropCatalogReq) -> Result<()> {
        let catalog_name = &req.name_ident.catalog_name;

        if catalog_name == CATALOG_DEFAULT {
            return Err(ErrorCode::BadArguments(
                "default catalog cannot be dropped".to_string(),
            ));
        }

        if self.external_catalogs.get(catalog_name).is_some() {
            return Err(ErrorCode::BadArguments(
                "catalog already exists that cannot be dropped".to_string(),
            ));
        }

        let _ = self.meta.drop_catalog(req).await;

        Ok(())
    }

    #[async_backtrace::framed]
    pub async fn list_catalogs(&self, tenant: &str) -> Result<Vec<Arc<dyn Catalog>>> {
        let mut catalogs = vec![self.get_default_catalog()?];

        // insert external catalogs.
        for ctl in self.external_catalogs.values() {
            catalogs.push(ctl.clone());
        }

        // fecth catalogs from metasrv.
        let infos = self.meta.list_catalogs(ListCatalogReq::new(tenant)).await?;

        for info in infos {
            catalogs.push(self.build_catalog(&info)?);
        }

        Ok(catalogs)
    }
}
