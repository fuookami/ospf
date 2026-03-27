import { AlgorithmShapeType, AxisType, LoadingPlanItemDTO, ShapeType } from './dto.ts';

export type ResolvedLoadingItemShape = {
  shapeType: ShapeType
  renderShapeType: ShapeType
  algorithmShapeType: AlgorithmShapeType
  axis: AxisType
  radius?: number
  diameter?: number
  boundingWidth: number
  boundingHeight: number
  boundingDepth: number
  actualVolume?: number | null
  unsupportedReason?: string
}

export function resolveLoadingItemShape(item: LoadingPlanItemDTO): ResolvedLoadingItemShape {
  const shapeType = item.shapeType ?? 'Cuboid';
  const renderShapeType = item.renderShapeType ?? shapeType;
  const algorithmShapeType = item.algorithmShapeType ?? 'Cuboid';
  const axis = item.axis ?? 'Y';
  const boundingWidth = item.boundingWidth ?? item.width;
  const boundingHeight = item.boundingHeight ?? item.height;
  const boundingDepth = item.boundingDepth ?? item.depth;

  if (renderShapeType === 'Cylinder') {
    const radius = item.radius ?? (item.diameter != null ? item.diameter / 2 : undefined);
    const diameter = item.diameter ?? (radius != null ? radius * 2 : undefined);

    if (radius == null || radius <= 0) {
      return {
        shapeType,
        renderShapeType,
        algorithmShapeType,
        axis,
        boundingWidth,
        boundingHeight,
        boundingDepth,
        actualVolume: item.actualVolume,
        unsupportedReason: `圆柱 ${item.name} 缺少有效半径或直径 / Cylinder ${item.name} has no valid radius or diameter`
      };
    }

    return {
      shapeType,
      renderShapeType,
      algorithmShapeType,
      axis,
      radius,
      diameter,
      boundingWidth,
      boundingHeight,
      boundingDepth,
      actualVolume: item.actualVolume
    };
  }

  return {
    shapeType,
    renderShapeType,
    algorithmShapeType,
    axis,
    boundingWidth,
    boundingHeight,
    boundingDepth,
    actualVolume: item.actualVolume
  };
}

export function shapeDisplayName(shape: ResolvedLoadingItemShape): string {
  return shape.renderShapeType;
}

export function cylinderAxisLength(shape: ResolvedLoadingItemShape): number {
  if (shape.axis === 'X') {
    return shape.boundingWidth;
  }
  if (shape.axis === 'Z') {
    return shape.boundingDepth;
  }
  return shape.boundingHeight;
}

export function formatSize(width: number, height: number, depth: number): string {
  return `${depth.toFixed(0)}*${width.toFixed(0)}*${height.toFixed(0)}`;
}
