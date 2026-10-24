# OSPF Rust 共享 BPP

[English](README.md)

维度无关的稀疏离散装载。`DiscreteLoadingContext` 将合法分配、整体占位和可选布局注册到
`MetaModel<V>`。重量使用 Quantity，通过显式 `PackingValueAdapter` 一次归一化；直到明确
solver 边界之前保持原数值类型。件数保持 `u64`，模型件数系数必须能精确转换。

`register_with` 将通用限制与下游变量、约束和目标一起原子注册。context 只属于一个模型且
禁止重复注册；重试或回退应创建全新模型和 context。`LoadingCapacityLimit` 使用真实有限
件数和重量容量，不使用任意 epsilon 或大 M。允许空预留；固定货物、外部固定占用及任何
严格正附加重量都必须整体占位。`link_additional_weight` 每舱位接受一个合并表达式并保存
用于结果提取，不能分别注册多份附加重量而漏算合计。

冲突边不具有传递性。`ConflictIndex` 贪心生成真实团，并保全所有原始边。仅对显式选择组
注册布局。`loading_conflicts` 纵向扫描候选区间，物理空间相互隔离，贴边不冲突，可通过
适配器进一步执行精确二维判交；共享层不依赖 BPP1D 或 BPP2D。

`extract_solution` 先复核包含扩展行的完整展开模型，再独立检查必须/固定装载、稀疏合法
分配、件数/重量容量、整体占位、冲突和布局。`verify` 可独立用于领域验证。通用列投影
属于 `ospf-rust-framework`，不在 BPP 内再建一套模型定义。

`PartitionLoadingContext` 要求舱位完整互斥归属、下游局部变量显式绑定；在通用定价之上
补充货物唯一性、跨区团的区内部分及局部布局兼容。完整配置列携带已验证领域元数据，
全轮 prepare 后原子提交投影和元数据。动态源行使旧注册失效，旧列不再可行时显式拒绝，
不在定价阶段漏掉新增业务规则。

`DiscreteLoadingApplicationService` 编排完整已注册源模型的同域 f64 普通 MILP；
`PartitionColumnGenerationApplicationService` 独立编排 Phase I、根定价、整轮批量加列
及无人工变量的受限整数 final。LP 原始／对偶证书与实际模型映射通过共享投影检查。
资源补全只释放显式归属且列入白名单的资源，冻结分配及占位轮廓，保留全部源行／
边界，排除旧列中资源的行与目标贡献；物化完整新列后再建 final。根与补全各有列上限，
共享总时限和取消句柄。
`contains_configuration` 使用同一归一化领域元数据检查候选，不分配列 ID；重复定价候选
不消耗新增列额度，避免前一个分区的重复列在额度上限处遮蔽后续分区的真正新列。
补全恢复整数资源以及全局／辅助整数的类型；整数资源出现小数时，完整向量复核失败，
不提交任何补全列。
CG 主报告仅认证源模型可行性，实际 `refinement_report` 与 `final_report` 独立保留。
补全回调返回时限时／取消，完整验证后的补全方案仍被保留，不再启动 final；补全／final
之间始终保留已验证的更好源方案。

`DiscreteLoadingBendersApplicationService` 独立编排共享固定追索算法。全部展开整数／辅助
整数及默认全部目标变量留在 master；显式连续目标追索必须由真实变量边界推导有限下界。
追索边界全部显式加入子问题行，保全对偶割与 Farkas 割的边界贡献。切割只来自本轮实际
LP 的独立复核证书。缺失／无效证据、模糊／不可行／无界终态、取消和预算耗尽分别记录。
可选普通 MILP 回退仅用剩余总预算和全新等价工厂；输入错误、取消和无剩余时间不启动回退。
回退不共享模型、context、池或对偶凭据；实际分解报告独立保留供审计。
结构化能力不支持、环境／许可证、后端及数值错误也可按显式策略回退；输入、建模、合同
及回调错误直接传播，不回退。原始可回退错误保留在 `decomposition_error`。限时／取消
报告携带的完整源可行解仍经验证保存，但终止后不继续求解或生成新切割。
回退无解或得到更差方案时不覆盖既有分解方案。选中方案在其自有源注册中提取，实际
全新求解保留在 `fallback_report`，不向新模型搬运旧 token 或受限 master 证明。

`DiscreteLoadingCertificationService` 在显式剩余预算内，用全新等价完整源 MILP 为加速结果
收尾。复核旧方案及源身份，检查新数值模型和领域绑定，仅用可行数值作 warm start。收尾无解、
限时方案更差或后端可恢复失败时保留最好方案；实际 `certification_report` 和
`certification_error` 分开保存。矛盾证明及合同错误仍被拒绝；取消／中断或预算耗尽不能启动
新求解。`validate` 仅复核原注册、源身份、完整向量和目标，不创建新模型、不调用后端、
不改写终止原因。调用方可独立选择是否收尾；启用时应预留时间。Demo2 通过显式
`source_milp_finishing` 策略接入，关闭时仍调用复核；源最优只来自实际完整源证明。
已有源方案后，CG final、证书恢复或 Benders fallback 的可恢复失败会连同
`acceleration_error`、`backend_error` 或 `decomposition_error` 保留该方案。换回更好的旧方案时
重建其逐行／逐变量审计并清除过期 trace 摘要；真实完整源 Unbounded 终态不会被有限方案覆盖。
已有完整源已验证终局证明时直接保留原报告，即使新预算为零或已取消，也不改写其 Completed
证明终态或再次求解。

受限 master 最优／不可行不等于 branch-and-price 或原领域证明。即使分解已验证收敛，
Benders 合并源报告仍保守为 Feasible，实际 master／sub 报告及有证据下界独立返回。全部
应用入口当前使用明确的同域 f64 callback，独立于 framework async feature，不接受跨域
精确 Benders。BPP1D 和 BPP2D 各自提供共享领域几何 adapter，互不依赖。
[Demo2](../ospf-rust-example/src/framework/demo2/README_ch.md) 的三条航空装载路径使用这些
共享服务，机型业务规则与结果提取仍保留在自身的领域 context 中。
