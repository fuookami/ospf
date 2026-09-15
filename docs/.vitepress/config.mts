import { defineConfig } from 'vitepress'
import { withMermaid } from 'vitepress-plugin-mermaid'

// https://vitepress.dev/reference/site-config
export default withMermaid({
  title: "ospf",
  description: "ospf reference document",
  base: '/ospf/',

  // Mermaid imports the CommonJS fastdom package from its ESM chunks. Pre-bundle
  // both entry points so Vite exposes the default export during `vitepress dev`.
  vite: {
    optimizeDeps: {
      include: [
        'fastdom',
        'fastdom/extensions/fastdom-promised.js'
      ]
    }
  },

  markdown: {
    math: true,
    lineNumbers: true,
    theme: {
      dark: "material-theme-darker",
      light: "material-theme-lighter"
    }
  },

  mermaid: {
    // Mermaid配置选项
  },

  mermaidPlugin: {
    class: 'mermaid'
  },

  locales: {
    root: {
      label: 'English',
      lang: 'en-us',
      themeConfig: {
        search: {
          provider: "local"
        },
        nav: [
          { text: 'Home', link: '/' },
          { text: 'Guide', link: '/guide/what-is-ospf' },
          { text: 'Examples', link: '/examples/example1' }
        ],
        sidebar: {
          '/guide': [
            {
              text: 'Introduction',
              items: [
                { text: 'What is OSPF?', link: '/guide/what-is-ospf' },
                { text: 'Getting Started', link: '/guide/getting-started' },
                { text: 'The Language of Operations Research', link: '/guide/operations-research-language' },
                { text: 'The Modeling and Solving Workflow', link: '/guide/modeling-workflow' },
                { text: 'Understanding Solver Results', link: '/guide/solving-results' },
                { text: 'Use Domain Driven Design Architecture', link: '/guide/use-ddd-architecture' }
              ]
            },
            {
              text: 'Advanced Applications',
              items: [
                {
                  text: 'Modeling',
                  collapsed: true,
                  items: [
                { 
                  text: 'Linear Function Symbols',
                  collapsed: true,
                  items: [
                    { text: 'Absolute Value', link: '/guide/linear-functional/abs' },
                    { text: 'Slack', link: '/guide/linear-functional/slack' },
                    { text: 'Slack Range', link: '/guide/linear-functional/slack-range' },
                    { text: 'Univariate Linear Piecewise', link: '/guide/linear-functional/ulp' },
                    { text: 'Bivariate Linear Piecewise', link: '/guide/linear-functional/blp' },
                    { text: 'Cosine', link: '/guide/linear-functional/cos' },
                    { text: 'Sine', link: '/guide/linear-functional/sin' },
                    { text: 'First Nonzero Index', link: '/guide/linear-functional/first' },
                    { text: 'Implication', link: '/guide/linear-functional/imply' },
                    { text: 'Inequality Indicator', link: '/guide/linear-functional/inequality' },
                    { text: 'In-Step Range', link: '/guide/linear-functional/in-step-range' },
                    { text: 'Same-As', link: '/guide/linear-functional/same-as' },
                    { text: 'Satisfied Amount', link: '/guide/linear-functional/satisfied-amount' },
                    { text: 'Satisfied-Amount Inequality', link: '/guide/linear-functional/satisfied-amount-inequality' },
                    { text: 'Sigmoid', link: '/guide/linear-functional/sigmoid' },
                    { text: 'Semi-Continuous Marker', link: '/guide/linear-functional/semi' },
                    { text: 'Masking', link: '/guide/linear-functional/masking' },
                    { text: 'Ceiling', link: '/guide/linear-functional/ceiling' },
                    { text: 'Floor', link: '/guide/linear-functional/floor' },
                    { text: 'Rounding', link: '/guide/linear-functional/rounding' },
                    { text: 'Modulo', link: '/guide/linear-functional/mod' },
                    { text: 'Minimum', link: '/guide/linear-functional/min' },
                    { text: 'Maximum', link: '/guide/linear-functional/max' },
                    { text: 'Binaryzation', link: '/guide/linear-functional/bin' },
                    { text: 'Balance Ternaryzation', link: '/guide/linear-functional/bter' },
                    { text: 'Logical AND', link: '/guide/linear-functional/and' },
                    { text: 'Logical OR', link: '/guide/linear-functional/or' },
                    { text: 'Logical NOT', link: '/guide/linear-functional/not' },
                    { text: 'Logical XOR', link: '/guide/linear-functional/xor' },
                    { text: 'Conditional IF', link: '/guide/linear-functional/if' },
                    { text: 'Conditional Interval', link: '/guide/linear-functional/if-in' },
                    { text: 'Conditional If-Then', link: '/guide/linear-functional/if-then' },
                    { text: 'One-of Constraint', link: '/guide/linear-functional/one-of' }
                  ]
                },
                { 
                  text: 'Quadratic Model Function Symbols',
                  collapsed: true,
                  items: [
                    { text: 'Slack (Linear Expressions)', link: '/guide/quadratic-functional/slack' },
                    { text: 'Slack Range (Linear Expressions)', link: '/guide/quadratic-functional/slack-range' },
                    { text: 'Positive Part', link: '/guide/quadratic-functional/positive-part' },
                    { text: 'Quadratic Product', link: '/guide/quadratic-functional/product' },
                    { text: 'Quadratic Linear', link: '/guide/quadratic-functional/quadratic-linear' },
                    { text: 'Quadratic In-Step Range', link: '/guide/quadratic-functional/quadratic-in-step-range' },
                    { text: 'Quadratic Masking Range', link: '/guide/quadratic-functional/quadratic-masking-range' },
                    { text: 'Quadratic Minimum', link: '/guide/quadratic-functional/quadratic-min' },
                  ]
                },
                { text: 'Constraint Programming and Global Constraints', link: '/guide/constraint-programming' },
                { text: 'Multi-Objective Optimization and Soft Constraints', link: '/guide/multi-objective-optimization' },
                { text: 'Numerical Stability and Modeling Quality', link: '/guide/numerical-modeling' }
                  ]
                },
                {
                  text: 'Language and Model Architecture',
                  collapsed: true,
                  items: [
                { text: 'Symbolic Expressions and Symbolic Operations', link: '/guide/symbolic-expressions' },
                { text: 'Compiler-Like Architecture and Model Transformation', link: '/guide/compiler-architecture' },
                { text: 'The Deductive Logic Expression of the Mathematical Model', link: '/guide/deductive-logic-expression' },
                { text: 'Formal Design and Formal Verification', link: '/guide/formal-design-and-formal-verification' }
                  ]
                },
                {
                  text: 'Solving Strategies',
                  collapsed: true,
                  items: [
                { text: 'Use Domain Driven Design Architecture With Column Generation Algorithm', link: '/guide/use-ddd-architecture-with-column-generation' },
                { text: 'Use Domain Driven Design Architecture With Benders Decomposition Algorithm', link: '/guide/use-ddd-architecture-with-benders' },
                { text: 'Rolling Optimization and Reoptimization', link: '/guide/rolling-optimization' }
                  ]
                },
                {
                  text: 'Diagnosis and Decision Analysis',
                  collapsed: true,
                  items: [
                { text: 'Infeasibility Analysis and Constraint Relaxation', link: '/guide/infeasibility-analysis' },
                { text: 'Automatic Critical Constraint Analysis', link: '/guide/critical-constraint-analysis' },
                { text: 'Scenario Analysis and Plan Comparison', link: '/guide/scenario-analysis' }
                  ]
                },
                {
                  text: 'Integration and Execution',
                  collapsed: true,
                  items: [
                { text: 'Remote Solver', link: '/guide/remote-solver' },
                { text: 'Time Slice Round Robin Solver', link: '/guide/time-slice-cycle-solver' },
                {
                  text: 'Integrating OSPF with LLMs',
                  collapsed: true,
                  items: [
                    { text: 'Business Path', link: '/guide/integrating-llms' },
                    { text: 'Technical Path', link: '/guide/integrating-llms-technical' }
                  ]
                }
                  ]
                }
              ]
            }
          ],
          '/examples': [
            {
              text: 'Simple Examples',
              items: [
                { text: 'Example 1: Assigning Problem', link: '/examples/example1' },
                { text: 'Example 2: Assigning Problem', link: '/examples/example2' },
                { text: 'Example 3: Ingredient Problem', link: '/examples/example3' },
                { text: 'Example 4: Ingredient Problem', link: '/examples/example4' },
                { text: 'Example 5: Knapsack Problem', link: '/examples/example5' },
                { text: 'Example 6: Knapsack Problem', link: '/examples/example6' },
                { text: 'Example 7: Transport Problem', link: '/examples/example7' },
                { text: 'Example 8: Production Problem', link: '/examples/example8' },
                { text: 'Example 9: Location Selection Problem', link: '/examples/example9' },
                { text: 'Example 10: Traveling Salesman Problem', link: '/examples/example10' },
                { text: 'Example 11: Max Flow Problem', link: '/examples/example11' },
                { text: 'Example 12: Portfolio Optimization Problem', link: '/examples/example12' },
                { text: 'Example 13: Two-Echelon Transport Problem', link: '/examples/example13' },
                { text: 'Example 14: Transshipment Problem', link: '/examples/example14' },
                { text: 'Example 15: Supply Transport Problem', link: '/examples/example15' },
                { text: 'Example 16: Production Inventory Problem', link: '/examples/example16' },
                { text: 'Example 17: Vehicle Routing Problem', link: '/examples/example17' },
              ]
            },
            {
              text: 'Complex Examples (with DDD Architecture)',
              items: [
                { text: 'Framework Example 1: Service Placement Problem', collapsed: true, items: [
                  { text: 'Overview', link: '/examples/framework-example1' },
                  { text: 'Route context model', link: '/examples/framework-example1/domain-route/domain-model' },
                  { text: 'Bandwidth context model', link: '/examples/framework-example1/domain-bandwidth/domain-model' }
                ] },
                { text: 'Framework Example 2: Aircraft Cargo Load Planning Problem (with Benders Decomposition)', collapsed: true, items: [
                  { text: 'Overview', link: '/examples/framework-example2' },
                  { text: 'Context model index', link: '/examples/framework-example2/domain-models' },
                  { text: 'Aircraft context model', link: '/examples/framework-example2/domain-aircraft/domain-model' },
                  { text: 'Stowage context model', link: '/examples/framework-example2/domain-stowage/domain-model' },
                  { text: 'MAC context model', link: '/examples/framework-example2/domain-mac/domain-model' },
                  { text: 'Airworthiness security context model', link: '/examples/framework-example2/domain-airworthiness_security/domain-model' },
                  { text: 'Soft security context model', link: '/examples/framework-example2/domain-soft_security/domain-model' },
                  { text: 'MAC optimization context model', link: '/examples/framework-example2/domain-mac_optimization/domain-model' },
                  { text: 'Express effectiveness context model', link: '/examples/framework-example2/domain-express_effectiveness/domain-model' },
                  { text: 'Loading effectiveness context model', link: '/examples/framework-example2/domain-loading_effectiveness/domain-model' },
                  { text: 'Redundancy context model', link: '/examples/framework-example2/domain-redundancy/domain-model' },
                  { text: 'Recommended weight equalization context model', link: '/examples/framework-example2/domain-recommended_weight_equalization/domain-model' },
                  { text: 'Payload maximization context model', link: '/examples/framework-example2/domain-payload_maximization/domain-model' }
                ] },
                { text: 'Framework Example 3: Cutting Stock Problem 1D (with Column Generation)', collapsed: true, items: [
                  { text: 'Overview', link: '/examples/framework-example3' },
                  { text: 'Material context model', link: '/examples/framework-example3/domain-material/domain-model' },
                  { text: 'Produce context model', link: '/examples/framework-example3/domain-produce/domain-model' }
                ] },
                { text: 'Framework Example 4: Flight Recovery Problem (with Column Generation)', collapsed: true, items: [
                  { text: 'Overview', link: '/examples/framework-example4' },
                  { text: 'Context model index', link: '/examples/framework-example4/domain-models' },
                  { text: 'Task context model', link: '/examples/framework-example4/domain-task/domain-model' },
                  { text: 'Rule context model', link: '/examples/framework-example4/domain-rule/domain-model' },
                  { text: 'Crew context model', link: '/examples/framework-example4/domain-crew/domain-model' },
                  { text: 'Cargo context model', link: '/examples/framework-example4/domain-cargo/domain-model' },
                  { text: 'Passenger context model', link: '/examples/framework-example4/domain-passenger/domain-model' },
                  { text: 'Bunch generation context model', link: '/examples/framework-example4/domain-bunch_generation/domain-model' },
                  { text: 'Bunch compilation context model', link: '/examples/framework-example4/domain-bunch_compilation/domain-model' },
                  { text: 'Bunch selection context model', link: '/examples/framework-example4/domain-bunch_selection/domain-model' }
                ] },
                { text: 'Framework Example 5: VRPTW Branch-and-Price', collapsed: true, items: [
                  { text: 'Overview', link: '/examples/framework-example5' },
                  { text: 'VRP context model', link: '/examples/framework-example5/domain-vrp/domain-model' },
                  { text: 'Route generation context model', link: '/examples/framework-example5/domain-route-generation/domain-model' },
                  { text: 'Route compilation context model', link: '/examples/framework-example5/domain-route-compilation/domain-model' }
                ] }
              ]
            }
          ]
        },
        socialLinks: [
          { icon: 'github', link: 'https://github.com/fuookami/ospf' }
        ],
        outline: {
          level: [2, 5]
        }
      }
    },
    'zh-cn': {
      label: '简体中文',
      lang: 'zh-cn',
      themeConfig: {
        outline: { label: '本页目录', level: [2, 5] },
        docFooter: { prev: '上一页', next: '下一页' },
        sidebarMenuLabel: '菜单',
        returnToTopLabel: '返回顶部',
        langMenuLabel: '切换语言',
        search: {
          provider: "local"
        },
        nav: [
          { text: '主页', link: '/zh-cn/' },
          { text: '指南', link: '/zh-cn/guide/what-is-ospf' },
          { text: '示例', link: '/zh-cn/examples/example1' }
        ],
        sidebar: {
          '/zh-cn/guide': [
            {
              text: '简介',
              items: [
                { text: 'OSPF 是什么?', link: '/zh-cn/guide/what-is-ospf' },
                { text: '快速开始', link: '/zh-cn/guide/getting-started' },
                { text: '运筹学领域语言', link: '/zh-cn/guide/operations-research-language' },
                { text: '建模与求解的完整流程', link: '/zh-cn/guide/modeling-workflow' },
                { text: '理解求解结果', link: '/zh-cn/guide/solving-results' },
                { text: '使用领域驱动设计架构', link: '/zh-cn/guide/use-ddd-architecture' }
              ]
            },
            {
              text: '高级应用',
              items: [
                {
                  text: '建模表达',
                  collapsed: true,
                  items: [
                { 
                  text: '线性函数符号',
                  collapsed: true,
                  items: [
                    { text: '绝对值', link: '/zh-cn/guide/linear-functional/abs' },
                    { text: '松弛', link: '/zh-cn/guide/linear-functional/slack' },
                    { text: '松弛（范围）', link: '/zh-cn/guide/linear-functional/slack-range' },
                    { text: '一元分段线性', link: '/zh-cn/guide/linear-functional/ulp' },
                    { text: '二元分段线性', link: '/zh-cn/guide/linear-functional/blp' },
                    { text: '余弦', link: '/zh-cn/guide/linear-functional/cos' },
                    { text: '正弦', link: '/zh-cn/guide/linear-functional/sin' },
                    { text: '首个非零索引', link: '/zh-cn/guide/linear-functional/first' },
                    { text: '蕴含', link: '/zh-cn/guide/linear-functional/imply' },
                    { text: '不等式指示函数', link: '/zh-cn/guide/linear-functional/inequality' },
                    { text: '步进区间', link: '/zh-cn/guide/linear-functional/in-step-range' },
                    { text: '同状态', link: '/zh-cn/guide/linear-functional/same-as' },
                    { text: '满足数量', link: '/zh-cn/guide/linear-functional/satisfied-amount' },
                    { text: '满足数量不等式', link: '/zh-cn/guide/linear-functional/satisfied-amount-inequality' },
                    { text: 'Sigmoid', link: '/zh-cn/guide/linear-functional/sigmoid' },
                    { text: '半连续标记', link: '/zh-cn/guide/linear-functional/semi' },
                    { text: '掩码函数', link: '/zh-cn/guide/linear-functional/masking' },
                    { text: '向上取整', link: '/zh-cn/guide/linear-functional/ceiling' },
                    { text: '向下取整', link: '/zh-cn/guide/linear-functional/floor' },
                    { text: '舍入', link: '/zh-cn/guide/linear-functional/rounding' },
                    { text: '取模', link: '/zh-cn/guide/linear-functional/mod' },
                    { text: '最小值', link: '/zh-cn/guide/linear-functional/min' },
                    { text: '最大值', link: '/zh-cn/guide/linear-functional/max' },
                    { text: '二值化', link: '/zh-cn/guide/linear-functional/bin' },
                    { text: '平衡三值化', link: '/zh-cn/guide/linear-functional/bter' },
                    { text: '逻辑与', link: '/zh-cn/guide/linear-functional/and' },
                    { text: '逻辑或', link: '/zh-cn/guide/linear-functional/or' },
                    { text: '逻辑非', link: '/zh-cn/guide/linear-functional/not' },
                    { text: '逻辑异或', link: '/zh-cn/guide/linear-functional/xor' },
                    { text: '条件 IF', link: '/zh-cn/guide/linear-functional/if' },
                    { text: '条件区间', link: '/zh-cn/guide/linear-functional/if-in' },
                    { text: '条件 If-Then', link: '/zh-cn/guide/linear-functional/if-then' },
                    { text: '选一约束', link: '/zh-cn/guide/linear-functional/one-of' }
                  ]
                },
                { 
                  text: '二次模型函数符号',
                  collapsed: true,
                  items: [
                    { text: '松弛（仅线性表达式）', link: '/zh-cn/guide/quadratic-functional/slack' },
                    { text: '松弛范围（仅线性表达式）', link: '/zh-cn/guide/quadratic-functional/slack-range' },
                    { text: '正部函数', link: '/zh-cn/guide/quadratic-functional/positive-part' },
                    { text: '二次乘积', link: '/zh-cn/guide/quadratic-functional/product' },
                    { text: '二次线性', link: '/zh-cn/guide/quadratic-functional/quadratic-linear' },
                    { text: '二次步进区间', link: '/zh-cn/guide/quadratic-functional/quadratic-in-step-range' },
                    { text: '二次掩码范围', link: '/zh-cn/guide/quadratic-functional/quadratic-masking-range' },
                    { text: '二次最小值', link: '/zh-cn/guide/quadratic-functional/quadratic-min' },
                  ]
                },
                { text: '约束规划与全局约束', link: '/zh-cn/guide/constraint-programming' },
                { text: '多目标优化与软约束', link: '/zh-cn/guide/multi-objective-optimization' },
                { text: '数值稳定性与建模质量', link: '/zh-cn/guide/numerical-modeling' }
                  ]
                },
                {
                  text: '语言与模型架构',
                  collapsed: true,
                  items: [
                { text: '符号表达式与符号运算', link: '/zh-cn/guide/symbolic-expressions' },
                { text: '类编译器架构与模型转换', link: '/zh-cn/guide/compiler-architecture' },
                { text: '数学模型的演绎逻辑表达', link: '/zh-cn/guide/deductive-logic-expression' },
                { text: '形式化设计与形式化验证', link: '/zh-cn/guide/formal-design-and-formal-verification' }
                  ]
                },
                {
                  text: '求解策略',
                  collapsed: true,
                  items: [
                { text: '使用领域驱动设计架构（列生成算法）', link: '/zh-cn/guide/use-ddd-architecture-with-column-generation' },
                { text: '使用领域驱动设计架构（Benders 分解算法）', link: '/zh-cn/guide/use-ddd-architecture-with-benders' },
                { text: '滚动优化与重优化', link: '/zh-cn/guide/rolling-optimization' }
                  ]
                },
                {
                  text: '诊断与决策分析',
                  collapsed: true,
                  items: [
                { text: '不可行性分析与约束放宽', link: '/zh-cn/guide/infeasibility-analysis' },
                { text: '临界约束自动分析', link: '/zh-cn/guide/critical-constraint-analysis' },
                { text: '场景分析与方案比较', link: '/zh-cn/guide/scenario-analysis' }
                  ]
                },
                {
                  text: '系统集成与执行',
                  collapsed: true,
                  items: [
                { text: '远端求解', link: '/zh-cn/guide/remote-solver' },
                { text: '时间片轮转求解器', link: '/zh-cn/guide/time-slice-cycle-solver' },
                {
                  text: 'OSPF 与 LLM 融合',
                  collapsed: true,
                  items: [
                    { text: '业务路径', link: '/zh-cn/guide/integrating-llms' },
                    { text: '技术路径', link: '/zh-cn/guide/integrating-llms-technical' }
                  ]
                }
                  ]
                }
              ]
            }
          ],
          '/zh-cn/examples': [
            {
              text: '简单示例',
              items: [
                { text: '示例 1：指派问题', link: '/zh-cn/examples/example1' },
                { text: '示例 2：指派问题', link: '/zh-cn/examples/example2' },
                { text: '示例 3：配料问题', link: '/zh-cn/examples/example3' },
                { text: '示例 4：配料问题', link: '/zh-cn/examples/example4' },
                { text: '示例 5：背包问题', link: '/zh-cn/examples/example5' },
                { text: '示例 6：背包问题', link: '/zh-cn/examples/example6' },
                { text: '示例 7：运输问题', link: '/zh-cn/examples/example7' },
                { text: '示例 8：生产问题', link: '/zh-cn/examples/example8' },
                { text: '示例 9：选址问题', link: '/zh-cn/examples/example9' },
                { text: '示例 10：旅行商问题', link: '/zh-cn/examples/example10' },
                { text: '示例 11：最大流问题', link: '/zh-cn/examples/example11' },
                { text: '示例 12：投资组合问题', link: '/zh-cn/examples/example12' },
                { text: '示例 13：二级运输问题', link: '/zh-cn/examples/example13' },
                { text: '示例 14：转运问题', link: '/zh-cn/examples/example14' },
                { text: '示例 15：供给运输问题', link: '/zh-cn/examples/example15' },
                { text: '示例 16：生产库存问题', link: '/zh-cn/examples/example16' },
                { text: '示例 17：车辆路径问题', link: '/zh-cn/examples/example17' },
              ]
            },
            {
              text: '复杂示例（使用领域驱动设计架构）',
              items: [
                { text: '复杂示例 1：服务放置问题', collapsed: true, items: [
                  { text: '总览', link: '/zh-cn/examples/framework-example1' },
                  { text: '路由上下文模型', link: '/zh-cn/examples/framework-example1/domain-route/domain-model' },
                  { text: '带宽上下文模型', link: '/zh-cn/examples/framework-example1/domain-bandwidth/domain-model' }
                ] },
                { text: '复杂示例 2：航空货运装载规划问题（使用 Benders 分解算法）', collapsed: true, items: [
                  { text: '总览', link: '/zh-cn/examples/framework-example2' },
                  { text: '上下文模型索引', link: '/zh-cn/examples/framework-example2/domain-models' },
                  { text: '飞机上下文模型', link: '/zh-cn/examples/framework-example2/domain-aircraft/domain-model' },
                  { text: '装载分配上下文模型', link: '/zh-cn/examples/framework-example2/domain-stowage/domain-model' },
                  { text: '平均气动弦（MAC）上下文模型', link: '/zh-cn/examples/framework-example2/domain-mac/domain-model' },
                  { text: '适航安全上下文模型', link: '/zh-cn/examples/framework-example2/domain-airworthiness_security/domain-model' },
                  { text: '软安全上下文模型', link: '/zh-cn/examples/framework-example2/domain-soft_security/domain-model' },
                  { text: 'MAC 优化上下文模型', link: '/zh-cn/examples/framework-example2/domain-mac_optimization/domain-model' },
                  { text: '快件效能上下文模型', link: '/zh-cn/examples/framework-example2/domain-express_effectiveness/domain-model' },
                  { text: '装载效能上下文模型', link: '/zh-cn/examples/framework-example2/domain-loading_effectiveness/domain-model' },
                  { text: '冗余上下文模型', link: '/zh-cn/examples/framework-example2/domain-redundancy/domain-model' },
                  { text: '建议载重量均衡上下文模型', link: '/zh-cn/examples/framework-example2/domain-recommended_weight_equalization/domain-model' },
                  { text: '载荷最大化上下文模型', link: '/zh-cn/examples/framework-example2/domain-payload_maximization/domain-model' }
                ] },
                { text: '复杂示例 3：一维分切问题（使用列生成算法）', collapsed: true, items: [
                  { text: '总览', link: '/zh-cn/examples/framework-example3' },
                  { text: '材料上下文模型', link: '/zh-cn/examples/framework-example3/domain-material/domain-model' },
                  { text: '生产上下文模型', link: '/zh-cn/examples/framework-example3/domain-produce/domain-model' }
                ] },
                { text: '复杂示例 4：航班恢复问题（使用列生成算法）', collapsed: true, items: [
                  { text: '总览', link: '/zh-cn/examples/framework-example4' },
                  { text: '上下文模型索引', link: '/zh-cn/examples/framework-example4/domain-models' },
                  { text: '任务上下文模型', link: '/zh-cn/examples/framework-example4/domain-task/domain-model' },
                  { text: '规则上下文模型', link: '/zh-cn/examples/framework-example4/domain-rule/domain-model' },
                  { text: '机组上下文模型', link: '/zh-cn/examples/framework-example4/domain-crew/domain-model' },
                  { text: '货物上下文模型', link: '/zh-cn/examples/framework-example4/domain-cargo/domain-model' },
                  { text: '旅客上下文模型', link: '/zh-cn/examples/framework-example4/domain-passenger/domain-model' },
                  { text: '航班串生成上下文模型', link: '/zh-cn/examples/framework-example4/domain-bunch_generation/domain-model' },
                  { text: '航班串编译上下文模型', link: '/zh-cn/examples/framework-example4/domain-bunch_compilation/domain-model' },
                  { text: '航班串选择上下文模型', link: '/zh-cn/examples/framework-example4/domain-bunch_selection/domain-model' }
                ] },
                { text: '复杂示例 5：VRPTW 分支定价', collapsed: true, items: [
                  { text: '总览', link: '/zh-cn/examples/framework-example5' },
                  { text: '车辆路径（VRP）上下文模型', link: '/zh-cn/examples/framework-example5/domain-vrp/domain-model' },
                  { text: '路线生成上下文模型', link: '/zh-cn/examples/framework-example5/domain-route-generation/domain-model' },
                  { text: '路线编译上下文模型', link: '/zh-cn/examples/framework-example5/domain-route-compilation/domain-model' }
                ] }
              ]
            }
          ]
        },
        socialLinks: [
          { icon: 'github', link: 'https://github.com/fuookami/ospf' }
        ]
      }
    }
  }
})
