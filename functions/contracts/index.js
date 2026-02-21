'use strict';

// =============================================================================
// Contract Schemas for all 7 Orchestrator Agents
// Ported from crates/agentics-contracts/src/
// =============================================================================

// ---------------------------------------------------------------------------
// 1. Workflow Orchestrator Agent
// ---------------------------------------------------------------------------
const workflowContract = {
  agent_id: 'workflow-orchestrator',
  agent_version: '0.1.0',
  classification: 'WORKFLOW_EXECUTION',
  description: 'Execute and coordinate multi-step workflows across agents in a deterministic and state-aware manner.',
  input: {
    type: 'object',
    required: ['workflow_id', 'workflow_name', 'tasks'],
    properties: {
      envelope_id: { type: 'string', format: 'uuid' },
      workflow_id: { type: 'string', format: 'uuid' },
      workflow_name: { type: 'string', minLength: 1, maxLength: 256 },
      workflow_version: { type: 'string' },
      state: { type: 'string', enum: ['pending', 'scheduled', 'running', 'paused', 'completed', 'failed', 'cancelled', 'retry_waiting'] },
      inputs: { type: 'object' },
      tasks: {
        type: 'array',
        items: {
          type: 'object',
          required: ['task_id', 'name', 'task_type'],
          properties: {
            task_id: { type: 'string', format: 'uuid' },
            name: { type: 'string', minLength: 1, maxLength: 256 },
            task_type: { type: 'string', enum: ['llm_completion', 'embedding', 'vector_search', 'transform', 'action', 'agent_invocation', 'conditional', 'parallel'] },
            state: { type: 'string', enum: ['pending', 'queued', 'running', 'completed', 'failed', 'skipped', 'cancelled', 'retry_waiting'] },
            inputs: { type: 'object' },
            outputs: { type: 'object' },
            depends_on: { type: 'array', items: { type: 'string', format: 'uuid' } },
            config: { type: 'object' },
          },
        },
      },
      config: {
        type: 'object',
        properties: {
          max_concurrency: { type: 'integer', default: 4 },
          timeout_seconds: { type: 'integer', default: 3600 },
          retry_config: {
            type: 'object',
            properties: {
              max_attempts: { type: 'integer', default: 3 },
              initial_delay_ms: { type: 'integer', default: 1000 },
              max_delay_ms: { type: 'integer', default: 30000 },
              backoff_multiplier: { type: 'number', default: 2.0 },
              jitter: { type: 'boolean', default: true },
            },
          },
          continue_on_failure: { type: 'boolean', default: false },
          strategy: { type: 'string', enum: ['sequential', 'parallel', 'dag_based', 'adaptive'] },
        },
      },
    },
  },
  output: {
    type: 'object',
    properties: {
      execution_id: { type: 'string' },
      repo_span: { type: 'object' },
      agent_spans: { type: 'array' },
      valid: { type: 'boolean' },
    },
  },
};

// ---------------------------------------------------------------------------
// 2. Task Scheduler Agent
// ---------------------------------------------------------------------------
const schedulerContract = {
  agent_id: 'task-scheduler',
  agent_version: '0.1.0',
  classification: 'TASK_COORDINATION',
  description: 'Schedule and coordinate task execution ordering, timing, and trigger-based invocation.',
  input: {
    type: 'object',
    required: ['schedule_id', 'tasks'],
    properties: {
      schedule_id: { type: 'string', format: 'uuid' },
      workflow_id: { type: 'string', format: 'uuid' },
      tasks: {
        type: 'array',
        items: {
          type: 'object',
          required: ['task_id', 'name', 'schedule_type'],
          properties: {
            task_id: { type: 'string' },
            name: { type: 'string' },
            schedule_type: { type: 'string', enum: ['immediate', 'delayed', 'scheduled', 'cron', 'event_triggered', 'dependency_based'] },
            trigger: {
              type: 'object',
              properties: {
                trigger_type: { type: 'string', enum: ['workflow_complete', 'step_complete', 'webhook', 'message_queue', 'external_api'] },
                condition: { type: 'object' },
              },
            },
            priority: { type: 'integer', default: 0 },
            delay_ms: { type: 'integer' },
            cron_expression: { type: 'string' },
            retry_policy: {
              type: 'object',
              properties: {
                max_retries: { type: 'integer', default: 3 },
                backoff_strategy: { type: 'string', enum: ['fixed', 'exponential', 'linear'] },
                initial_delay_ms: { type: 'integer', default: 1000 },
              },
            },
          },
        },
      },
      config: {
        type: 'object',
        properties: {
          max_concurrent: { type: 'integer', default: 10 },
          timeout_ms: { type: 'integer', default: 300000 },
        },
      },
    },
  },
  output: {
    type: 'object',
    properties: {
      schedule_id: { type: 'string' },
      status: { type: 'string', enum: ['scheduled', 'running', 'completed', 'failed'] },
      scheduled_tasks: { type: 'array' },
      next_execution: { type: 'string', format: 'date-time' },
    },
  },
};

// ---------------------------------------------------------------------------
// 3. Dependency Resolver Agent
// ---------------------------------------------------------------------------
const dependenciesContract = {
  agent_id: 'dependency-resolver',
  agent_version: '0.1.0',
  classification: 'TASK_COORDINATION',
  description: 'Analyze task dependencies and determine execution order with cycle detection and critical path analysis.',
  input: {
    type: 'object',
    required: ['request_id', 'workflow_id', 'tasks'],
    properties: {
      request_id: { type: 'string', format: 'uuid' },
      workflow_id: { type: 'string', format: 'uuid' },
      tasks: {
        type: 'array',
        items: {
          type: 'object',
          required: ['task_id', 'name'],
          properties: {
            task_id: { type: 'string' },
            name: { type: 'string' },
            task_type: { type: 'string' },
            dependencies: {
              type: 'array',
              items: {
                type: 'object',
                properties: {
                  task_id: { type: 'string' },
                  required_status: { type: 'string', enum: ['success', 'complete', 'started', 'skipped', 'any'] },
                  timeout_ms: { type: 'integer' },
                },
              },
            },
            estimated_duration_ms: { type: 'integer' },
            priority: { type: 'integer', default: 0 },
          },
        },
      },
      config: {
        type: 'object',
        properties: {
          max_depth: { type: 'integer', default: 100 },
          detect_cycles: { type: 'boolean', default: true },
          calculate_critical_path: { type: 'boolean', default: true },
          optimize_parallelism: { type: 'boolean', default: true },
          timeout_ms: { type: 'integer' },
        },
      },
      timestamp: { type: 'string', format: 'date-time' },
    },
  },
  output: {
    type: 'object',
    properties: {
      request_id: { type: 'string' },
      workflow_id: { type: 'string' },
      status: { type: 'string', enum: ['success', 'success_with_warnings', 'failed', 'timeout', 'cancelled'] },
      execution_order: { type: 'array', items: { type: 'string' } },
      parallel_groups: {
        type: 'array',
        items: {
          type: 'object',
          properties: {
            group_index: { type: 'integer' },
            task_ids: { type: 'array', items: { type: 'string' } },
            depends_on_groups: { type: 'array', items: { type: 'integer' } },
          },
        },
      },
      graph_summary: {
        type: 'object',
        properties: {
          total_tasks: { type: 'integer' },
          total_dependencies: { type: 'integer' },
          max_depth: { type: 'integer' },
          root_tasks: { type: 'integer' },
          leaf_tasks: { type: 'integer' },
          parallel_groups: { type: 'integer' },
          has_cycles: { type: 'boolean' },
        },
      },
      critical_path: {
        type: 'object',
        properties: {
          task_ids: { type: 'array', items: { type: 'string' } },
          total_duration_ms: { type: 'integer' },
          task_count: { type: 'integer' },
        },
      },
      issues: { type: 'array' },
      resolved_at: { type: 'string', format: 'date-time' },
    },
  },
};

// ---------------------------------------------------------------------------
// 4. Retry & Recovery Agent
// ---------------------------------------------------------------------------
const retryContract = {
  agent_id: 'retry-recovery',
  agent_version: '0.1.0',
  classification: 'TASK_COORDINATION',
  description: 'Determine safe retry, backoff, or recovery strategies following task or workflow failure.',
  input: {
    type: 'object',
    required: ['request_id', 'failure'],
    properties: {
      request_id: { type: 'string', format: 'uuid' },
      workflow_id: { type: 'string', format: 'uuid' },
      execution_id: { type: 'string', format: 'uuid' },
      failure: {
        type: 'object',
        required: ['task_id', 'error_category'],
        properties: {
          task_id: { type: 'string' },
          task_name: { type: 'string' },
          error_category: { type: 'string', enum: ['transient', 'permanent', 'timeout', 'resource_exhaustion', 'rate_limit', 'validation', 'dependency', 'infrastructure', 'unknown'] },
          error_message: { type: 'string' },
          error_code: { type: 'string' },
          attempt: { type: 'integer' },
          max_attempts: { type: 'integer' },
          last_attempt_at: { type: 'string', format: 'date-time' },
          context: { type: 'object' },
        },
      },
      retry_policy: {
        type: 'object',
        properties: {
          max_attempts: { type: 'integer', default: 3 },
          backoff_strategy: { type: 'string', enum: ['fixed', 'exponential', 'linear', 'fibonacci', 'immediate'], default: 'exponential' },
          initial_delay_ms: { type: 'integer', default: 1000 },
          max_delay_ms: { type: 'integer', default: 30000 },
          backoff_multiplier: { type: 'number', default: 2.0 },
          jitter: { type: 'boolean', default: true },
        },
      },
      timestamp: { type: 'string', format: 'date-time' },
    },
  },
  output: {
    type: 'object',
    properties: {
      request_id: { type: 'string' },
      decision: { type: 'string', enum: ['retry', 'retry_with_modification', 'skip', 'fail', 'rollback', 'alternate_path', 'defer_retry'] },
      retry_delay_ms: { type: 'integer' },
      next_attempt: { type: 'integer' },
      modifications: { type: 'object' },
      recovery_path: { type: 'string' },
      confidence: { type: 'number', minimum: 0, maximum: 1 },
      reasoning: { type: 'string' },
      timestamp: { type: 'string', format: 'date-time' },
    },
  },
};

// ---------------------------------------------------------------------------
// 5. Parallelization Agent
// ---------------------------------------------------------------------------
const parallelContract = {
  agent_id: 'parallelization-agent',
  agent_version: '0.1.0',
  classification: 'TASK_COORDINATION',
  description: 'Identify tasks that can execute concurrently to improve workflow throughput.',
  input: {
    type: 'object',
    required: ['request_id', 'workflow_id', 'tasks'],
    properties: {
      request_id: { type: 'string', format: 'uuid' },
      workflow_id: { type: 'string', format: 'uuid' },
      tasks: {
        type: 'array',
        items: {
          type: 'object',
          required: ['task_id', 'name'],
          properties: {
            task_id: { type: 'string' },
            name: { type: 'string' },
            task_type: { type: 'string' },
            dependencies: { type: 'array' },
            estimated_duration_ms: { type: 'integer' },
            priority: { type: 'integer', default: 0 },
          },
        },
      },
      config: {
        type: 'object',
        properties: {
          max_concurrency: { type: 'integer', default: 8 },
          respect_soft_dependencies: { type: 'boolean', default: true },
          analyze_data_dependencies: { type: 'boolean', default: true },
          consider_resource_constraints: { type: 'boolean', default: true },
          optimize_makespan: { type: 'boolean', default: true },
          balance_load: { type: 'boolean', default: false },
          timeout_ms: { type: 'integer' },
        },
      },
      resource_constraints: {
        type: 'object',
        properties: {
          max_total_memory_bytes: { type: 'integer' },
          max_total_cpu: { type: 'number' },
          max_total_gpu: { type: 'number' },
          max_total_tokens: { type: 'integer' },
        },
      },
      timestamp: { type: 'string', format: 'date-time' },
    },
  },
  output: {
    type: 'object',
    properties: {
      request_id: { type: 'string' },
      workflow_id: { type: 'string' },
      status: { type: 'string', enum: ['success', 'no_opportunities', 'success_with_warnings', 'failed', 'timeout'] },
      execution_plan: {
        type: 'object',
        properties: {
          plan_id: { type: 'string' },
          phases: { type: 'array' },
          total_phases: { type: 'integer' },
          max_parallelism: { type: 'integer' },
          strategy: { type: 'string', enum: ['sequential', 'static_parallel', 'dynamic_parallel', 'pipeline', 'hybrid'] },
          critical_path_tasks: { type: 'array', items: { type: 'string' } },
        },
      },
      summary: {
        type: 'object',
        properties: {
          total_tasks: { type: 'integer' },
          parallelizable_tasks: { type: 'integer' },
          sequential_tasks: { type: 'integer' },
          execution_phases: { type: 'integer' },
          max_parallelism: { type: 'integer' },
          avg_parallelism: { type: 'number' },
        },
      },
      optimization_metrics: {
        type: 'object',
        properties: {
          sequential_duration_ms: { type: 'integer' },
          parallel_duration_ms: { type: 'integer' },
          speedup_factor: { type: 'number' },
          efficiency_score: { type: 'number' },
        },
      },
      issues: { type: 'array' },
      analyzed_at: { type: 'string', format: 'date-time' },
    },
  },
};

// ---------------------------------------------------------------------------
// 6. State Machine Agent
// ---------------------------------------------------------------------------
const stateMachineContract = {
  agent_id: 'state-machine-agent',
  agent_version: '1.0.0',
  classification: 'STATE_MANAGEMENT',
  description: 'Manage workflow state transitions according to defined state machines and rules.',
  input: {
    type: 'object',
    required: ['request_id', 'execution_id', 'entity_type', 'current_state', 'target_state', 'reason', 'initiated_by'],
    properties: {
      request_id: { type: 'string', format: 'uuid' },
      execution_id: { type: 'string', format: 'uuid' },
      entity_type: { type: 'string', enum: ['workflow', 'task'] },
      current_state: { type: 'string' },
      target_state: { type: 'string' },
      reason: { type: 'string', minLength: 1, maxLength: 1024 },
      initiated_by: { type: 'string' },
      force: { type: 'boolean', default: false },
      context: { type: 'object' },
      timestamp: { type: 'string', format: 'date-time' },
    },
  },
  output: {
    type: 'object',
    properties: {
      request_id: { type: 'string' },
      execution_id: { type: 'string' },
      entity_type: { type: 'string' },
      success: { type: 'boolean' },
      previous_state: { type: 'string' },
      new_state: { type: 'string' },
      status: { type: 'string', enum: ['completed', 'invalid', 'blocked', 'forced', 'no_change', 'error'] },
      validation: {
        type: 'object',
        properties: {
          valid: { type: 'boolean' },
          source_state_matches: { type: 'boolean' },
          target_state_exists: { type: 'boolean' },
          rule_violations: { type: 'array' },
          warnings: { type: 'array' },
        },
      },
      confidence: { type: 'number', minimum: 0, maximum: 1 },
      timestamp: { type: 'string', format: 'date-time' },
    },
  },
  state_machine: {
    workflow: {
      initial_state: 'pending',
      terminal_states: ['completed', 'cancelled'],
      transitions: {
        pending: ['scheduled', 'running', 'cancelled'],
        scheduled: ['running', 'cancelled'],
        running: ['paused', 'completed', 'failed', 'cancelled', 'retry_waiting'],
        paused: ['running', 'cancelled'],
        retry_waiting: ['running', 'failed', 'cancelled'],
        failed: ['retry_waiting'],
      },
    },
    task: {
      initial_state: 'pending',
      terminal_states: ['completed', 'skipped', 'cancelled'],
      transitions: {
        pending: ['queued', 'running', 'skipped', 'cancelled'],
        queued: ['running', 'cancelled'],
        running: ['completed', 'failed', 'cancelled', 'retry_waiting'],
        retry_waiting: ['running', 'failed', 'cancelled'],
        failed: ['retry_waiting'],
      },
    },
  },
};

// ---------------------------------------------------------------------------
// 7. Swarm Coordinator Agent
// ---------------------------------------------------------------------------
const swarmContract = {
  agent_id: 'swarm-coordinator-agent',
  agent_version: '0.1.0',
  classification: 'TASK_COORDINATION',
  description: 'Coordinate fan-out / fan-in execution across multiple agents working toward a shared objective.',
  input: {
    type: 'object',
    required: ['request_id', 'workflow_id', 'objective', 'workers'],
    properties: {
      request_id: { type: 'string', format: 'uuid' },
      workflow_id: { type: 'string', format: 'uuid' },
      objective: {
        type: 'object',
        required: ['id', 'description', 'objective_type'],
        properties: {
          id: { type: 'string' },
          description: { type: 'string' },
          objective_type: { type: 'string', enum: ['research', 'analysis', 'generation', 'execution', 'review'] },
          inputs: { type: 'object' },
          success_criteria: { type: 'array' },
          priority: { type: 'integer', minimum: 0, maximum: 10 },
        },
      },
      workers: {
        type: 'array',
        items: {
          type: 'object',
          required: ['worker_id', 'agent_type', 'task'],
          properties: {
            worker_id: { type: 'string' },
            agent_type: { type: 'string', enum: ['workflow_orchestrator', 'dependency_resolver', 'parallelization'] },
            task: {
              type: 'object',
              required: ['task_id', 'description'],
              properties: {
                task_id: { type: 'string' },
                description: { type: 'string' },
                inputs: { type: 'object' },
                expected_outputs: { type: 'array', items: { type: 'string' } },
                timeout_ms: { type: 'integer' },
              },
            },
            depends_on: { type: 'array', items: { type: 'string' } },
            config: {
              type: 'object',
              properties: {
                max_retries: { type: 'integer', default: 3 },
                retry_delay_ms: { type: 'integer', default: 1000 },
                critical: { type: 'boolean', default: false },
              },
            },
          },
        },
      },
      config: {
        type: 'object',
        properties: {
          max_concurrent_workers: { type: 'integer', default: 8 },
          coordination_timeout_ms: { type: 'integer', default: 300000 },
          consensus_strategy: { type: 'string', enum: ['first_wins', 'last_wins', 'majority', 'weighted_majority', 'merge_all'] },
          continue_on_partial_failure: { type: 'boolean', default: false },
          convergence_threshold: { type: 'number', default: 0.8, minimum: 0, maximum: 1 },
        },
      },
      aggregation: {
        type: 'object',
        properties: {
          mode: { type: 'string', enum: ['combine', 'select_best', 'summarize', 'collect'], default: 'combine' },
          aggregate_fields: { type: 'array', items: { type: 'string' } },
          merge_fields: { type: 'array', items: { type: 'string' } },
        },
      },
      timestamp: { type: 'string', format: 'date-time' },
    },
  },
  output: {
    type: 'object',
    properties: {
      request_id: { type: 'string' },
      workflow_id: { type: 'string' },
      status: { type: 'string', enum: ['success', 'partial_success', 'consensus_reached', 'failed', 'timeout', 'cancelled'] },
      aggregated_output: {
        type: 'object',
        properties: {
          primary: {},
          fields: { type: 'object' },
          consensus: { type: 'object' },
          confidence: { type: 'number' },
        },
      },
      worker_results: { type: 'array' },
      summary: {
        type: 'object',
        properties: {
          total_workers: { type: 'integer' },
          successful_workers: { type: 'integer' },
          failed_workers: { type: 'integer' },
          max_concurrency_achieved: { type: 'integer' },
          total_duration_ms: { type: 'integer' },
          objective_met: { type: 'boolean' },
        },
      },
      issues: { type: 'array' },
      completed_at: { type: 'string', format: 'date-time' },
    },
  },
};

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------
const AGENT_CONTRACTS = {
  workflow: workflowContract,
  scheduler: schedulerContract,
  dependencies: dependenciesContract,
  retry: retryContract,
  parallel: parallelContract,
  'state-machine': stateMachineContract,
  swarm: swarmContract,
};

module.exports = { AGENT_CONTRACTS };
