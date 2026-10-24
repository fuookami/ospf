import lodash from "lodash";
import type { SchemaDTO } from "./dto.ts";

export type GanttSubItemVO = {
    name: string
    category: string
    startTime: string
    endTime: string
    info: Map<string, string>
}

export type GanttItemVO = {
    name: string
    category: string
    executor: string
    subItems: GanttSubItemVO[]
    scheduledStartTime: string
    scheduledEndTime: string
    startTime: string
    endTime: string
    renderStartTime: string
    renderEndTime: string
    produces: Map<string, string>
    consumption: Map<string, string>
    info: Map<string, string>
}

export type GanttLineVO = {
    name: string
    category: string,
    items: GanttItemVO[]
}

export type GanttChartVO = {
    startTime: string
    endTime: string
    linkInfo: string[]
    lines: GanttLineVO[]
}

export function dump(schema: SchemaDTO): GanttChartVO | null {
    const items: GanttItemVO[] = schema.tasks.flatMap(task => {
        if (task.startTime == null || task.endTime == null) return [];
        const scheduledStartTime = task.scheduledStartTime ?? task.startTime;
        const scheduledEndTime = task.scheduledEndTime ?? task.endTime;
        const produces = new Map(Object.entries(task.products ?? {}));
        if (task.order != null) produces.set("order", task.order);
        if (task.produce != null) produces.set("produce", task.produce);
        const consumption = new Map(Object.entries({...task.materials, ...task.consumption}));
        const info = new Map(Object.entries({...task.resources, ...task.info}));
        return [{
            name: task.name,
            category: task.category,
            executor: task.executor,
            subItems: task.subTasks.flatMap(subTask => {
                if (subTask.startTime == null || subTask.endTime == null) return [];
                return [{
                    name: subTask.name,
                    category: subTask.category,
                    startTime: subTask.startTime,
                    endTime: subTask.endTime,
                    info: new Map(Object.entries(subTask.info))
                }];
            }),
            scheduledStartTime,
            scheduledEndTime,
            startTime: task.startTime,
            endTime: task.endTime,
            renderStartTime: lodash.min([scheduledStartTime, task.startTime]) as string,
            renderEndTime: lodash.max([scheduledEndTime, task.endTime]) as string,
            produces,
            consumption,
            info
        }];
    });
    if (items.length === 0) return null;
    const executors = lodash.uniq(items.map(item => item.executor));
    const lines: GanttLineVO[] = executors.map(executor => ({
        name: executor,
        category: "Normal",
        items: items.filter(item => item.executor === executor).sort((lhs, rhs) =>
            lhs.renderStartTime.localeCompare(rhs.renderStartTime))
    }));
    return {
        startTime: lodash.minBy(items, "renderStartTime")!.renderStartTime,
        endTime: lodash.maxBy(items, "renderEndTime")!.renderEndTime,
        linkInfo: lodash.uniq(lodash.flatMap(schema.tasks, task => Object.keys(task.resources))),
        lines
    };
}
