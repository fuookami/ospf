export const offset: number = 80;

export type CuttingPlanProductionView = {
    name: string
    x: number
    displayFromX: string
    displayToX: string
    width: number
    displayWidth: string
    productionType: string
    info: Map<string, string>
    color: string | null
}

export type CuttingPlanProductionDTO = {
    id?: string | null
    name: string
    x: number
    width: number
    productionType: string
    unitLength?: number | null
    amount?: number | null
    info: Record<string, string>
    color?: string | null
}

export type CuttingPlanDTO = {
    id?: string | null
    materialId?: string | null
    group: string[]
    productions: CuttingPlanProductionDTO[]
    width: number
    standardWidth: number
    restWidth?: number | null
    amount: number
    info: Record<string, string>
}

export type SchemaDTO = {
    kpi: Record<string, string>
    cuttingPlans: CuttingPlanDTO[]
}
