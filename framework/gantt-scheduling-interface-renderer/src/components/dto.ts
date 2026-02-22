export type GanttSubTaskDTO = {
    name: string
    category: string
    startTime: string
    endTime: string
    info: Map<string, string>
}

export type GanttTaskDTO = {
    name: string
    category: string
    startTime: string
    endTime: string
    resources: Object | Map<string, string>
    info: Object | Map<string, string>
    executor: string
    order: string | null
    produce: string | null
    products: Object | Map<string, string> | null
    materials: Object | Map<string, string> | null
    consumption: Object | Map<string, string>
    scheduledStartTime: string | null
    scheduledEndTime: string | null
    subTasks: GanttSubTaskDTO[]
}

export type SchemaDTO = {
    tasks: GanttTaskDTO[]
}
