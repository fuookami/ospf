export type ShapeType = 'Cuboid' | 'Cylinder'

export type AlgorithmShapeType = 'Cuboid' | 'VerticalCylinder' | 'HorizontalCylinderX' | 'HorizontalCylinderZ' | 'BoundingCuboid'

export type AxisType = 'X' | 'Y' | 'Z'

export type InfoDTO = Record<string, string>

export type LoadingPlanItemDTO = {
    name: string
    packageType?: string | null
    width: number
    height: number
    depth: number
    x: number
    y: number
    z: number
    weight: number
    loadingOrder: number
    shapeType?: ShapeType | null
    renderShapeType?: ShapeType | null
    algorithmShapeType?: AlgorithmShapeType | null
    radius?: number | null
    diameter?: number | null
    axis?: AxisType | null
    boundingWidth?: number | null
    boundingHeight?: number | null
    boundingDepth?: number | null
    actualVolume?: number | null
    info?: InfoDTO | null
}

export type LoadingPlanDTO = {
    group: string[]
    name: string
    typeCode: string
    width: number
    height: number
    depth: number
    loadingRate: number
    weight: number
    volume: number
    items: LoadingPlanItemDTO[]
    info?: InfoDTO | null
}

export type SchemaDTO = {
    kpi?: InfoDTO | null,
    loadingPlans: LoadingPlanDTO[]
}
