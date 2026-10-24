import { test } from 'node:test';
import assert from 'node:assert/strict';
import { dump } from '../src/components/vo.ts';
import type { GanttTaskDTO } from '../src/components/dto.ts';

function task(overrides: Partial<GanttTaskDTO> = {}): GanttTaskDTO {
    return {
        name: 'A', category: 'Normal', executor: 'M', order: '0', produce: null,
        products: { '0': 'product' }, consumption: { steel: '2' }, materials: { oil: '1' },
        resources: { operator: 'Alice' }, info: {},
        startTime: '2026-10-24 08:00:00.000000000', endTime: '2026-10-24 09:00:00.000000000',
        scheduledStartTime: null, scheduledEndTime: null,
        subTasks: [{ name: 'setup', category: 'Normal', startTime: '2026-10-24 08:00:00.000000000',
            endTime: '2026-10-24 08:10:00.000000000', info: { note: 'setup' } }],
        ...overrides
    };
}

test('normalized Kotlin/Rust records become view maps without losing consumption or order zero', () => {
    const chart = dump({ tasks: [task()] })!;
    const item = chart.lines[0].items[0];
    assert.equal(item.produces.get('order'), '0');
    assert.equal(item.produces.get('0'), 'product');
    assert.equal(item.consumption.get('steel'), '2');
    assert.equal(item.consumption.get('oil'), '1');
    assert.equal(item.info.get('operator'), 'Alice');
    assert.equal(item.subItems[0].info.get('note'), 'setup');
    assert.deepEqual(chart.linkInfo, ['operator']);
});

test('empty and unscheduled data have no fabricated chart dates', () => {
    assert.equal(dump({ tasks: [] }), null);
    assert.equal(dump({ tasks: [task({ startTime: null, endTime: null })] }), null);
    const chart = dump({ tasks: [task({ startTime: null, endTime: null }), task()] })!;
    assert.equal(chart.lines[0].items.length, 1);
});

test('baseline range expands chart and untimed subtasks are not drawn', () => {
    const chart = dump({ tasks: [task({
        scheduledStartTime: '2026-10-24 07:00:00.000000000',
        subTasks: [{ name: 'pending', category: 'Unknown', startTime: null, endTime: null, info: {} }]
    })] })!;
    assert.equal(chart.startTime, '2026-10-24 07:00:00.000000000');
    assert.equal(chart.lines[0].items[0].subItems.length, 0);
});
