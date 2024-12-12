export type LoadingPlanItem = {
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
    color: string | null
}

export type LoadingPlan = {
    group: string[]
    typeCode: string
    width: number
    height: number
    depth: number
    loadingRate: number
    weight: number
    volume: number
    items: LoadingPlanItem[]
    info: Map<string, string>
}

export type Schema = {
    kpi: Map<string, string>,
    loadingPlan: LoadingPlan[]
}
