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

use std::sync::Arc;

use common_exception::Result;
use common_expression::DataBlock;
use common_expression::DataField;
use common_pipeline_core::processors::port::OutputPort;
use common_pipeline_core::Pipeline;
use common_pipeline_core::SourcePipeBuilder;
use common_pipeline_sources::OneBlockSource;
use common_profile::SharedProcessorProfiles;

use super::processors::transforms::hash_join::HashJoinBuildState;
use crate::api::DefaultExchangeInjector;
use crate::api::ExchangeInjector;

#[derive(Clone)]
pub struct PipelineBuilderData {
    pub input_join_state: Option<Arc<HashJoinBuildState>>,
    pub input_probe_schema: Option<Vec<DataField>>,
}

pub struct PipelineBuildResult {
    pub main_pipeline: Pipeline,
    // Containing some sub queries pipelines, must be complete pipeline
    pub sources_pipelines: Vec<Pipeline>,

    /// Set of profiling spans for the query.
    /// Will be empty if profiling is disabled.
    pub prof_span_set: SharedProcessorProfiles,

    pub exchange_injector: Arc<dyn ExchangeInjector>,
    /// for local fragment data sharing
    pub builder_data: PipelineBuilderData,
}

impl PipelineBuildResult {
    pub fn create() -> PipelineBuildResult {
        PipelineBuildResult {
            main_pipeline: Pipeline::create(),
            sources_pipelines: vec![],
            prof_span_set: SharedProcessorProfiles::default(),
            exchange_injector: DefaultExchangeInjector::create(),
            builder_data: PipelineBuilderData {
                input_join_state: None,
                input_probe_schema: None,
            },
        }
    }

    pub fn from_blocks(blocks: Vec<DataBlock>) -> Result<PipelineBuildResult> {
        let mut source_builder = SourcePipeBuilder::create();

        for data_block in blocks {
            let output = OutputPort::create();
            source_builder.add_source(output.clone(), OneBlockSource::create(output, data_block)?);
        }

        let mut main_pipeline = Pipeline::create();
        main_pipeline.add_pipe(source_builder.finalize());

        Ok(PipelineBuildResult {
            main_pipeline,
            sources_pipelines: vec![],
            prof_span_set: SharedProcessorProfiles::default(),
            exchange_injector: DefaultExchangeInjector::create(),
            builder_data: PipelineBuilderData {
                input_join_state: None,
                input_probe_schema: None,
            },
        })
    }

    pub fn set_max_threads(&mut self, max_threads: usize) {
        self.main_pipeline.set_max_threads(max_threads);

        for source_pipeline in &mut self.sources_pipelines {
            source_pipeline.set_max_threads(max_threads);
        }
    }
}
