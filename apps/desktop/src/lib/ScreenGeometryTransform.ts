/**
 * FarmerPro - ScreenGeometryTransform
 * Unified screen geometry and coordinate mapping engine.
 * Solves FP-002, FP-003: Landscape in portrait container, object-contain letterboxing,
 * pillarboxing, zoom levels, and out-of-bounds click rejection.
 */

export interface RenderedImageBounds {
  /** Offset from container left to the rendered image left in pixels */
  offsetX: number;
  /** Offset from container top to the rendered image top in pixels */
  offsetY: number;
  /** Actual width of the rendered image inside the container in pixels */
  renderedWidth: number;
  /** Actual height of the rendered image inside the container in pixels */
  renderedHeight: number;
  /** Container width in pixels */
  containerWidth: number;
  /** Container height in pixels */
  containerHeight: number;
  /** Scale factor applied to image (renderedWidth / imageWidth) */
  scale: number;
  /** Aspect ratio of the device image */
  imageAspect: number;
  /** Aspect ratio of the container */
  containerAspect: number;
}

export interface DevicePoint {
  deviceX: number;
  deviceY: number;
  normX: number; // 0.0 to 1.0
  normY: number; // 0.0 to 1.0
}

export interface DeviceBounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface OverlayCssStyle {
  left: string;
  top: string;
  width: string;
  height: string;
}

export class ScreenGeometryTransform {
  /**
   * Calculates the exact pixel rect where the device image is displayed inside
   * an `object-fit: contain` container.
   */
  public static calculateRenderedBounds(
    containerWidth: number,
    containerHeight: number,
    imageWidth: number,
    imageHeight: number
  ): RenderedImageBounds {
    if (containerWidth <= 0 || containerHeight <= 0 || imageWidth <= 0 || imageHeight <= 0) {
      return {
        offsetX: 0,
        offsetY: 0,
        renderedWidth: 0,
        renderedHeight: 0,
        containerWidth,
        containerHeight,
        scale: 1,
        imageAspect: 1,
        containerAspect: 1,
      };
    }

    const imageAspect = imageWidth / imageHeight;
    const containerAspect = containerWidth / containerHeight;

    let renderedWidth: number;
    let renderedHeight: number;
    let offsetX: number;
    let offsetY: number;

    if (imageAspect > containerAspect) {
      // Image is wider than container -> pillarbox top and bottom (letterbox)
      renderedWidth = containerWidth;
      renderedHeight = containerWidth / imageAspect;
      offsetX = 0;
      offsetY = (containerHeight - renderedHeight) / 2;
    } else {
      // Image is taller than container -> pillarbox left and right
      renderedHeight = containerHeight;
      renderedWidth = containerHeight * imageAspect;
      offsetY = 0;
      offsetX = (containerWidth - renderedWidth) / 2;
    }

    const scale = renderedWidth / imageWidth;

    return {
      offsetX,
      offsetY,
      renderedWidth,
      renderedHeight,
      containerWidth,
      containerHeight,
      scale,
      imageAspect,
      containerAspect,
    };
  }

  /**
   * Converts client (browser mouse/touch) coordinates into physical device coordinates.
   * If the click falls into black letterbox/pillarbox borders, returns `null` (rejection).
   */
  public static clientToDevice(
    clientX: number,
    clientY: number,
    containerRect: { left: number; top: number; width: number; height: number },
    imageWidth: number,
    imageHeight: number
  ): DevicePoint | null {
    const rendered = this.calculateRenderedBounds(
      containerRect.width,
      containerRect.height,
      imageWidth,
      imageHeight
    );

    const relX = clientX - containerRect.left;
    const relY = clientY - containerRect.top;

    // Check letterbox / pillarbox boundaries
    if (
      relX < rendered.offsetX ||
      relX > rendered.offsetX + rendered.renderedWidth ||
      relY < rendered.offsetY ||
      relY > rendered.offsetY + rendered.renderedHeight
    ) {
      return null; // Click fell into black border
    }

    // Normalized inside rendered image [0.0, 1.0]
    const normX = Math.max(0, Math.min(1, (relX - rendered.offsetX) / rendered.renderedWidth));
    const normY = Math.max(0, Math.min(1, (relY - rendered.offsetY) / rendered.renderedHeight));

    const deviceX = Math.round(normX * imageWidth);
    const deviceY = Math.round(normY * imageHeight);

    return {
      deviceX: Math.max(0, Math.min(imageWidth, deviceX)),
      deviceY: Math.max(0, Math.min(imageHeight, deviceY)),
      normX,
      normY,
    };
  }

  /**
   * Converts physical device coordinates into container pixel coordinates.
   */
  public static deviceToContainer(
    deviceX: number,
    deviceY: number,
    containerWidth: number,
    containerHeight: number,
    imageWidth: number,
    imageHeight: number
  ): { containerX: number; containerY: number } {
    const rendered = this.calculateRenderedBounds(
      containerWidth,
      containerHeight,
      imageWidth,
      imageHeight
    );

    const normX = imageWidth > 0 ? deviceX / imageWidth : 0;
    const normY = imageHeight > 0 ? deviceY / imageHeight : 0;

    const containerX = rendered.offsetX + normX * rendered.renderedWidth;
    const containerY = rendered.offsetY + normY * rendered.renderedHeight;

    return { containerX, containerY };
  }

  /**
   * Transforms physical device element bounds (x, y, width, height) into exact CSS style
   * (left, top, width, height) relative to the container element, accounting for letterboxing.
   */
  public static deviceBoundsToOverlayStyle(
    bounds: DeviceBounds,
    containerWidth: number,
    containerHeight: number,
    imageWidth: number,
    imageHeight: number,
    usePercentage = true
  ): OverlayCssStyle {
    const rendered = this.calculateRenderedBounds(
      containerWidth,
      containerHeight,
      imageWidth,
      imageHeight
    );

    if (imageWidth <= 0 || imageHeight <= 0 || rendered.renderedWidth <= 0) {
      return { left: '0px', top: '0px', width: '0px', height: '0px' };
    }

    const normX = bounds.x / imageWidth;
    const normY = bounds.y / imageHeight;
    const normW = bounds.width / imageWidth;
    const normH = bounds.height / imageHeight;

    const pixelLeft = rendered.offsetX + normX * rendered.renderedWidth;
    const pixelTop = rendered.offsetY + normY * rendered.renderedHeight;
    const pixelWidth = normW * rendered.renderedWidth;
    const pixelHeight = normH * rendered.renderedHeight;

    if (usePercentage && containerWidth > 0 && containerHeight > 0) {
      return {
        left: `${(pixelLeft / containerWidth) * 100}%`,
        top: `${(pixelTop / containerHeight) * 100}%`,
        width: `${(pixelWidth / containerWidth) * 100}%`,
        height: `${(pixelHeight / containerHeight) * 100}%`,
      };
    }

    return {
      left: `${Math.round(pixelLeft)}px`,
      top: `${Math.round(pixelTop)}px`,
      width: `${Math.round(pixelWidth)}px`,
      height: `${Math.round(pixelHeight)}px`,
    };
  }

  /**
   * Converts a user-dragged selection box in container coordinates to physical device bounds.
   * Clamps the selection strictly to the active rendered image area (excluding black bars).
   */
  public static containerRegionToDeviceBounds(
    startContainerX: number,
    startContainerY: number,
    endContainerX: number,
    endContainerY: number,
    containerWidth: number,
    containerHeight: number,
    imageWidth: number,
    imageHeight: number
  ): DeviceBounds | null {
    const rendered = this.calculateRenderedBounds(
      containerWidth,
      containerHeight,
      imageWidth,
      imageHeight
    );

    const minX = Math.min(startContainerX, endContainerX);
    const maxX = Math.max(startContainerX, endContainerX);
    const minY = Math.min(startContainerY, endContainerY);
    const maxY = Math.max(startContainerY, endContainerY);

    // Clamp to rendered image rect
    const clampedMinX = Math.max(rendered.offsetX, Math.min(rendered.offsetX + rendered.renderedWidth, minX));
    const clampedMaxX = Math.max(rendered.offsetX, Math.min(rendered.offsetX + rendered.renderedWidth, maxX));
    const clampedMinY = Math.max(rendered.offsetY, Math.min(rendered.offsetY + rendered.renderedHeight, minY));
    const clampedMaxY = Math.max(rendered.offsetY, Math.min(rendered.offsetY + rendered.renderedHeight, maxY));

    const w = clampedMaxX - clampedMinX;
    const h = clampedMaxY - clampedMinY;

    if (w < 2 || h < 2) {
      return null; // Region too small or entirely in letterbox
    }

    const normX = (clampedMinX - rendered.offsetX) / rendered.renderedWidth;
    const normY = (clampedMinY - rendered.offsetY) / rendered.renderedHeight;
    const normW = w / rendered.renderedWidth;
    const normH = h / rendered.renderedHeight;

    return {
      x: Math.round(normX * imageWidth),
      y: Math.round(normY * imageHeight),
      width: Math.round(normW * imageWidth),
      height: Math.round(normH * imageHeight),
    };
  }

  /**
   * Leaf-first hit testing in TypeScript.
   * Prioritizes interactive leaf controls (Button, CheckBox, text nodes) over
   * large container layouts (pad_numeric, ViewGroup, LinearLayout).
   */
  public static findNodeAtPoint(
    nodes: Array<{
      id: number;
      bounds: DeviceBounds;
      is_visible?: boolean;
      clickable?: boolean;
      checkable?: boolean;
      focusable?: boolean;
      children_ids?: number[];
      text?: string;
      resource_id?: string;
      class_name?: string;
      depth?: number;
    }>,
    devX: number,
    devY: number
  ): any | null {
    const candidates = nodes.filter(
      (n) =>
        n.is_visible !== false &&
        devX >= n.bounds.x &&
        devX <= n.bounds.x + n.bounds.width &&
        devY >= n.bounds.y &&
        devY <= n.bounds.y + n.bounds.height
    );

    if (candidates.length === 0) return null;

    candidates.sort((a, b) => {
      const score = (n: typeof a): number => {
        let s = 0;
        if (n.clickable || n.checkable || n.focusable) s += 10000;
        if (!n.children_ids || n.children_ids.length === 0) s += 5000;
        if (n.text || n.resource_id) s += 3000;
        const cls = n.class_name || '';
        if (
          cls.includes('Button') ||
          cls.includes('Edit') ||
          cls.includes('Check') ||
          cls.includes('Switch') ||
          cls.includes('Text')
        ) {
          s += 2000;
        }
        s += (n.depth || 0) * 500;
        const area = (n.bounds.width || 0) * (n.bounds.height || 0);
        s -= Math.min(8000, area / 50);
        return s;
      };

      return score(b) - score(a);
    });

    return candidates[0];
  }
}

