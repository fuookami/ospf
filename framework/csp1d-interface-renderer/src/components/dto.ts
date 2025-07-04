export const offset: number = 80;

export type CuttingPlanProduction = {
    name: string
    x: number
    width: number
    productionType: string
    info: Map<string, string>
    color: string | null
}

export type CuttingPlan = {
    group: string[]
    productions: CuttingPlanProduction[]
    width: number
    standardWidth: number
    amount: number
    info: Map<string, string>
}

export type Schema = {
    kpi: Map<string, string>
    cuttingPlans: CuttingPlan[]
}
