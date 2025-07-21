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
    resources: Map<string, string>
    info: Map<string, string>
    executor: string
    order: string | null
    produce: string | null
    products: Map<string, string> | null
    materials: Map<string, string> | null
    consumption: Map<string, string>
    scheduledStartTime: string | null
    scheduledEndTime: string | null
    subTasks: GanttSubTaskDTO[]
}

export type SchemaDTO = {
    tasks: GanttTaskDTO[]
}
