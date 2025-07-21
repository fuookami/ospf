export const offset: number = 80;

export type CuttingPlanProductionDTO = {
    name: string
    x: number
    width: number
    productionType: string
    info: Map<string, string>
    color: string | null
}

export type CuttingPlanDTO = {
    group: string[]
    productions: CuttingPlanProductionDTO[]
    width: number
    standardWidth: number
    amount: number
    info: Map<string, string>
}

export type SchemaDTO = {
    kpi: Map<string, string>
    cuttingPlans: CuttingPlanDTO[]
}
