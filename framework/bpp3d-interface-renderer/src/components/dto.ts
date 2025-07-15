export type LoadingPlanItemDTO = {
    name: string
    packageType: string
    width: number
    height: number
    depth: number
    x: number
    y: number
    z: number
    weight: number
    loadingOrder: number
    info: Map<string, string>
}

export type LoadingPlanDTO = {
    group: string[]
    typeCode: string
    width: number
    height: number
    depth: number
    loadingRate: number
    weight: number
    volume: number
    items: LoadingPlanItemDTO[]
    info: Map<string, string>
}

export type SchemaDTO = {
    kpi: Map<string, string>,
    loadingPlan: LoadingPlanDTO[]
}
