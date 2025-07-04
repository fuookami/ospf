export type GanttSubItem = {
    name: string
    category: string,
    startTime: string,
    endTime: string,
    info: Map<string, string>
}

export type GanttItem = {
    name: string
    category: string,
    subItems: GanttSubItem[],
    scheduledStartTime: string,
    scheduledEndTime: string,
    startTime: string,
    endTime: string,
    produces: Map<string, string>,
    consumption: Map<string, string>,
    info: Map<string, string>
}

export type GanttLine = {
    name: string
    category: string,
    items: GanttItem[]
}

export type Schema = {
    startTime: string
    endTime: string
    linkInfo: string[]
    lines: GanttLine[]
}
