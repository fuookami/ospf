export type GanttSubTaskDTO = {
    name: string
    category: string
    startTime: string | null
    endTime: string | null
    info: Record<string, string>
}

export type GanttTaskDTO = {
    name: string
    category: string
    startTime: string | null
    endTime: string | null
    resources: Record<string, string>
    info: Record<string, string>
    executor: string
    order: string | null
    produce: string | null
    products: Record<string, string> | null
    materials: Record<string, string> | null
    consumption: Record<string, string>
    scheduledStartTime: string | null
    scheduledEndTime: string | null
    subTasks: GanttSubTaskDTO[]
}

export type SchemaDTO = {
    tasks: GanttTaskDTO[]
}
