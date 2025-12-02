export class GerberViewer {
  constructor() {
    // Main canvas (WebGL2)
    this.canvas = document.getElementById("gerber-canvas");
    this.gl = null; // WebGL2 context

    // DOM elements
    this.fileInput = document.getElementById("file-input");
    this.selectFilesBtn = document.getElementById("select-files-btn");
    this.fitViewBtn = document.querySelector(".button-container button");
    this.selectAllBtn = document.getElementById("select-all-btn");
    this.unselectAllBtn = document.getElementById("unselect-all-btn");
    this.clearAllBtn = document.getElementById("clear-all-btn");
    this.alphaSlider = document.getElementById("alpha-slider");
    this.alphaValue = document.getElementById("alpha-value");
    this.layerList = document.getElementById("layer-list");

    // Drawer elements
    this.drawer = document.getElementById("drawer");
    this.resizeHandle = document.getElementById("resize-handle");
    this.drawerToggleBtn = document.getElementById("drawer-toggle");

    // WASM module and single processor
    this.wasmModule = null;
    this.wasmProcessor = null;

    // Layers
    this.layers = [];

    // Camera
    this.camera = {
      zoom: 1.0,
      offsetX: 0.0,
      offsetY: 0.0,
    };

    // Interaction
    this.isPanning = false;
    this.lastMousePos = { x: 0, y: 0 };

    // Touch interaction
    this.isTouching = false;
    this.touches = [];
    this.initialPinchDistance = null;
    this.lastPinchDistance = null;
    this.lastTouchCenter = { x: 0, y: 0 };

    // Drawer resize state
    this.isResizingDrawer = false;
    this.drawerCurrentWidth = 300; // Default width
    this.drawerMinWidth = 200;
    this.drawerMaxWidth = 600;

    // Colors
    this.colorPalette = [
      [1.0, 0.0, 0.0], // Red
      [0.0, 1.0, 0.0], // Green
      [0.0, 0.0, 1.0], // Blue
      [1.0, 1.0, 0.0], // Yellow
      [1.0, 0.0, 1.0], // Magenta
      [0.0, 1.0, 1.0], // Cyan
    ];
    this.nextColorIndex = 0;

    // Global alpha
    this.globalAlpha = 0.7;
  }

  async init() {
    // Load WASM module
    this.wasmModule = await import("../wasm/pkg/wasm_gerber_processor.js");
    await this.wasmModule.default();
    this.wasmModule.init_panic_hook();

    // Create WebGL2 context
    this.gl = this.canvas.getContext("webgl2");
    if (!this.gl) {
      throw new Error("WebGL2 not supported");
    }

    // Initialize Gerber processor
    this.wasmProcessor = new this.wasmModule.GerberProcessor();
    this.wasmProcessor.init(this.gl);

    // Resize Canvas
    this.resizeCanvas();
    window.addEventListener("resize", () => this.resizeCanvas());

    this.setupEventListeners();

    // Initial render
    this.render();
  }

  resizeCanvas() {
    const rect = this.canvas.getBoundingClientRect();
    this.canvas.width = rect.width;
    this.canvas.height = rect.height;

    if (this.wasmProcessor) {
      this.wasmProcessor.resize();
    }

    this.render();
  }

  setupEventListeners() {
    // File input
    this.selectFilesBtn.addEventListener("click", () => {
      this.fileInput.click();
    });

    this.fileInput.addEventListener("change", (e) => {
      if (e.target.files.length > 0) {
        this.handleFileUpload(e.target.files);
      }
    });

    // Fit view button
    this.fitViewBtn.addEventListener("click", () => {
      this.fitView();
    });

    // Layer control buttons
    this.selectAllBtn.addEventListener("click", () => {
      this.selectAllLayerCheckboxes();
    });

    this.unselectAllBtn.addEventListener("click", () => {
      this.unselectAllLayerCheckboxes();
    });

    this.clearAllBtn.addEventListener("click", () => {
      this.clearAllLayers();
    });

    // Alpha slider
    this.alphaSlider.addEventListener("input", (e) => {
      const alpha = parseInt(e.target.value) / 100;
      this.alphaValue.textContent = `${e.target.value}%`;
      this.updateGlobalAlpha(alpha);
    });

    // Canvas mouse events
    this.canvas.addEventListener("mousedown", (e) => this.handleMouseDown(e));
    this.canvas.addEventListener("mousemove", (e) => this.handleMouseMove(e));
    this.canvas.addEventListener("mouseup", (e) => this.handleMouseUp(e));
    this.canvas.addEventListener("mouseleave", (e) => this.handleMouseUp(e));
    this.canvas.addEventListener("wheel", (e) => this.handleWheel(e));

    // Canvas touch events
    this.canvas.addEventListener("touchstart", (e) => this.handleTouchStart(e), {
      passive: false,
    });
    this.canvas.addEventListener("touchmove", (e) => this.handleTouchMove(e), {
      passive: false,
    });
    this.canvas.addEventListener("touchend", (e) => this.handleTouchEnd(e), {
      passive: false,
    });
    this.canvas.addEventListener("touchcancel", (e) => this.handleTouchEnd(e), {
      passive: false,
    });

    // Drawer resize events (mouse)
    this.resizeHandle.addEventListener("mousedown", (e) =>
      this.startDrawerResize(e),
    );
    document.addEventListener("mousemove", (e) => this.resizeDrawer(e));
    document.addEventListener("mouseup", (e) => this.stopDrawerResize(e));

    // Drawer resize events (touch)
    this.resizeHandle.addEventListener(
      "touchstart",
      (e) => this.startDrawerResize(e),
      { passive: false },
    );
    document.addEventListener("touchmove", (e) => this.resizeDrawer(e), {
      passive: false,
    });
    document.addEventListener("touchend", (e) => this.stopDrawerResize(e), {
      passive: false,
    });

    // Drawer toggle event
    this.drawerToggleBtn.addEventListener("click", (e) => {
      e.stopPropagation();
      e.preventDefault();
      this.toggleDrawer();
    });
  }

  async handleFileUpload(files) {
    // Process all files in parallel (skip rendering during parallel processing)
    const promises = Array.from(files).map(async (file) => {
      try {
        const content = await file.text();
        await this.addLayer(file.name, content);
      } catch (error) {
        console.error(`Failed to load file ${file.name}:`, error);
      }
    });

    await Promise.all(promises);

    // Render once after all layers are added
    this.renderLayerList();
    this.render();
    this.fitView();

    // Clear file input
    this.fileInput.value = "";
  }

  async addLayer(name, content) {
    try {
      // add layer to WASM processor
      const layerId = this.wasmProcessor.add_layer(content);

      // Get layer boundary from WASM
      const bounds = this.wasmProcessor.get_boundary();

      const color =
        this.colorPalette[this.nextColorIndex % this.colorPalette.length];
      this.nextColorIndex++;

      const layer = {
        id: `layer-${layerId}`,
        layerId: layerId, // WASM layer_id
        name: name,
        visible: true,
        color: color,
        bounds: {
          minX: bounds.min_x,
          maxX: bounds.max_x,
          minY: bounds.min_y,
          maxY: bounds.max_y,
        },
      };

      this.layers.push(layer);
    } catch (error) {
      console.error(`[Layer] Failed to add layer ${name}:`, error);
      throw error;
    }
  }

  render() {
    if (!this.wasmProcessor) return;

    // Get selected layers
    const selectedLayerIds = this.getSelectedLayerIds();

    const activeLayerIds = [];
    const colorData = [];

    this.layers.forEach((layer) => {
      if (selectedLayerIds.has(layer.id)) {
        activeLayerIds.push(layer.layerId);

        // Add RGB color (no alpha)
        colorData.push(layer.color[0]);
        colorData.push(layer.color[1]);
        colorData.push(layer.color[2]);
      }
    });

    // Set active layers
    this.wasmProcessor.set_active_layers(
      new Uint32Array(activeLayerIds),
      new Float32Array(colorData),
    );

    this.wasmProcessor.render(
      this.camera.zoom,
      this.camera.zoom,
      this.camera.offsetX,
      this.camera.offsetY,
      this.globalAlpha,
    );
  }

  getSelectedLayerIds() {
    const selectedIds = new Set();
    this.layers.forEach((layer) => {
      if (layer.visible) {
        selectedIds.add(layer.id);
      }
    });
    return selectedIds;
  }

  fitView() {
    // Get selected layer IDs
    const selectedLayerIds = this.getSelectedLayerIds();

    if (selectedLayerIds.size === 0) {
      return;
    }

    // Get selected layers
    const selectedLayers = this.layers.filter((layer) =>
      selectedLayerIds.has(layer.id),
    );

    if (selectedLayers.length === 0) {
      return;
    }

    // Calculate combined bounds from all selected layers
    let minX = Infinity;
    let maxX = -Infinity;
    let minY = Infinity;
    let maxY = -Infinity;

    for (const layer of selectedLayers) {
      if (layer.bounds) {
        minX = Math.min(minX, layer.bounds.minX);
        maxX = Math.max(maxX, layer.bounds.maxX);
        minY = Math.min(minY, layer.bounds.minY);
        maxY = Math.max(maxY, layer.bounds.maxY);
      }
    }

    if (
      !isFinite(minX) ||
      !isFinite(maxX) ||
      !isFinite(minY) ||
      !isFinite(maxY)
    ) {
      return;
    }

    const boundsWidth = maxX - minX;
    const boundsHeight = maxY - minY;
    const centerX = (minX + maxX) / 2;
    const centerY = (minY + maxY) / 2;

    if (this.canvas.width === 0 || this.canvas.height === 0) {
      return;
    }

    if (boundsWidth === 0 && boundsHeight === 0) {
      // Can't fit to a single point, just center it
      this.camera.zoom = 2.0;
      this.camera.offsetX = -centerX * this.camera.zoom;
      this.camera.offsetY = -centerY * this.camera.zoom;
      this.render();
      return;
    }

    const canvasAspect = this.canvas.width / this.canvas.height;

    let zoom;
    if (boundsWidth === 0) {
      zoom = (2.0 / boundsHeight) * 0.9;
    } else if (boundsHeight === 0) {
      zoom = (2.0 / boundsWidth) * 0.9;
    } else {
      const boundsAspect = boundsWidth / boundsHeight;
      if (boundsAspect > canvasAspect) {
        zoom = (2.0 / boundsWidth) * 0.9;
      } else {
        zoom = (2.0 / boundsHeight) * 0.9;
      }
    }

    this.camera.zoom = zoom;
    this.camera.offsetX = -centerX * zoom;
    this.camera.offsetY = -centerY * zoom;

    this.render();
  }

  handleWheel(e) {
    e.preventDefault();

    const rect = this.canvas.getBoundingClientRect();
    if (rect.width === 0 || rect.height === 0) {
      return;
    }

    const mx_screen = e.clientX - rect.left;
    const my_screen = e.clientY - rect.top;

    const centerX = rect.width / 2;
    const centerY = rect.height / 2;
    const mouseXNDC = ((mx_screen - centerX) / rect.width) * 2;
    const mouseYNDC = -((my_screen - centerY) / rect.height) * 2;

    const aspect = this.canvas.width / this.canvas.height;

    // Apply aspect ratio correction to mouse position
    let mouseXCorrected, mouseYCorrected;
    if (aspect > 1.0) {
      mouseXCorrected = mouseXNDC * aspect;
      mouseYCorrected = mouseYNDC;
    } else {
      mouseXCorrected = mouseXNDC;
      mouseYCorrected = mouseYNDC / aspect;
    }

    const zoomChange = 1 + (e.deltaY<0?1:-1) * Math.sqrt(Math.abs(e.deltaY)) * 0.02;

    const prevZoom = this.camera.zoom;
    const newZoom = prevZoom * zoomChange;

    const zoomRatio = newZoom / prevZoom;

    this.camera.offsetX =
      (this.camera.offsetX - mouseXCorrected) * zoomRatio + mouseXCorrected;
    this.camera.offsetY =
      (this.camera.offsetY - mouseYCorrected) * zoomRatio + mouseYCorrected;
    this.camera.zoom = newZoom;

    this.render();
  }

  handleMouseDown(e) {
    if (e.button === 2) return; // Ignore right-click
    this.isPanning = true;
    this.lastMousePos.x = e.clientX;
    this.lastMousePos.y = e.clientY;
  }

  handleMouseMove(e) {
    if (!this.isPanning) return;

    const deltaX = e.clientX - this.lastMousePos.x;
    const deltaY = e.clientY - this.lastMousePos.y;

    // Visual feedback during drag
    this.canvas.style.transform = `translate(${deltaX}px, ${deltaY}px)`;
  }

  handleMouseUp(e) {
    if (!this.isPanning) return;

    this.isPanning = false;

    const canvasRect = this.canvas.getBoundingClientRect();
    if (canvasRect.width === 0 || canvasRect.height === 0) {
      return;
    }

    const deltaX = e.clientX - this.lastMousePos.x;
    const deltaY = e.clientY - this.lastMousePos.y;

    // Reset transform
    this.canvas.style.transform = "";

    const deltaXNDC = (deltaX / canvasRect.width) * 2;
    const deltaYNDC = (-deltaY / canvasRect.height) * 2;
    const aspect = this.canvas.width / this.canvas.height;

    if (aspect > 1.0) {
      this.camera.offsetX += deltaXNDC * aspect;
      this.camera.offsetY += deltaYNDC;
    } else {
      this.camera.offsetX += deltaXNDC;
      this.camera.offsetY += deltaYNDC / aspect;
    }

    this.render();
  }

  // Touch event handlers
  handleTouchStart(e) {
    e.preventDefault();

    this.isTouching = true;
    this.touches = Array.from(e.touches);

    if (this.touches.length === 2) {
      // Two-finger gesture: pinch-to-zoom
      this.initialPinchDistance = this.calculateTouchDistance(
        this.touches[0],
        this.touches[1],
      );
      this.lastPinchDistance = this.initialPinchDistance;

      const center = this.getTouchCenter(this.touches[0], this.touches[1]);
      this.lastTouchCenter = center;
    } else if (this.touches.length === 1) {
      // Single finger: pan
      this.lastTouchCenter = {
        x: this.touches[0].clientX,
        y: this.touches[0].clientY,
      };
    }
  }

  handleTouchMove(e) {
    e.preventDefault();

    if (!this.isTouching) return;

    this.touches = Array.from(e.touches);

    if (this.touches.length === 2) {
      // Two-finger gesture: pinch-to-zoom + pan
      const currentDistance = this.calculateTouchDistance(
        this.touches[0],
        this.touches[1],
      );
      const currentCenter = this.getTouchCenter(
        this.touches[0],
        this.touches[1],
      );

      // Handle pinch zoom
      if (this.lastPinchDistance !== null) {
        const rect = this.canvas.getBoundingClientRect();
        if (rect.width === 0 || rect.height === 0) return;

        // Calculate zoom center in NDC
        const mx_screen = currentCenter.x - rect.left;
        const my_screen = currentCenter.y - rect.top;

        const centerX = rect.width / 2;
        const centerY = rect.height / 2;
        const mouseXNDC = ((mx_screen - centerX) / rect.width) * 2;
        const mouseYNDC = -((my_screen - centerY) / rect.height) * 2;

        const aspect = this.canvas.width / this.canvas.height;

        let mouseXCorrected, mouseYCorrected;
        if (aspect > 1.0) {
          mouseXCorrected = mouseXNDC * aspect;
          mouseYCorrected = mouseYNDC;
        } else {
          mouseXCorrected = mouseXNDC;
          mouseYCorrected = mouseYNDC / aspect;
        }

        // Apply zoom
        const zoomChange = currentDistance / this.lastPinchDistance;
        const prevZoom = this.camera.zoom;
        const newZoom = prevZoom * zoomChange;
        const zoomRatio = newZoom / prevZoom;

        this.camera.offsetX =
          (this.camera.offsetX - mouseXCorrected) * zoomRatio + mouseXCorrected;
        this.camera.offsetY =
          (this.camera.offsetY - mouseYCorrected) * zoomRatio + mouseYCorrected;
        this.camera.zoom = newZoom;

        this.lastPinchDistance = currentDistance;
      }

      // Handle pan
      const deltaX = currentCenter.x - this.lastTouchCenter.x;
      const deltaY = currentCenter.y - this.lastTouchCenter.y;

      const canvasRect = this.canvas.getBoundingClientRect();
      if (canvasRect.width > 0 && canvasRect.height > 0) {
        const deltaXNDC = (deltaX / canvasRect.width) * 2;
        const deltaYNDC = (-deltaY / canvasRect.height) * 2;
        const aspect = this.canvas.width / this.canvas.height;

        if (aspect > 1.0) {
          this.camera.offsetX += deltaXNDC * aspect;
          this.camera.offsetY += deltaYNDC;
        } else {
          this.camera.offsetX += deltaXNDC;
          this.camera.offsetY += deltaYNDC / aspect;
        }
      }

      this.lastTouchCenter = currentCenter;
      this.render();
    } else if (this.touches.length === 1) {
      // Single finger: pan
      const currentPos = {
        x: this.touches[0].clientX,
        y: this.touches[0].clientY,
      };

      const deltaX = currentPos.x - this.lastTouchCenter.x;
      const deltaY = currentPos.y - this.lastTouchCenter.y;

      const canvasRect = this.canvas.getBoundingClientRect();
      if (canvasRect.width > 0 && canvasRect.height > 0) {
        const deltaXNDC = (deltaX / canvasRect.width) * 2;
        const deltaYNDC = (-deltaY / canvasRect.height) * 2;
        const aspect = this.canvas.width / this.canvas.height;

        if (aspect > 1.0) {
          this.camera.offsetX += deltaXNDC * aspect;
          this.camera.offsetY += deltaYNDC;
        } else {
          this.camera.offsetX += deltaXNDC;
          this.camera.offsetY += deltaYNDC / aspect;
        }
      }

      this.lastTouchCenter = currentPos;
      this.render();
    }
  }

  handleTouchEnd(e) {
    e.preventDefault();

    this.touches = Array.from(e.touches);

    if (this.touches.length < 2) {
      // Reset pinch state
      this.initialPinchDistance = null;
      this.lastPinchDistance = null;
    }

    if (this.touches.length === 0) {
      // All touches ended
      this.isTouching = false;
    } else if (this.touches.length === 1) {
      // Transitioned from multi-touch to single touch
      this.lastTouchCenter = {
        x: this.touches[0].clientX,
        y: this.touches[0].clientY,
      };
    }
  }

  calculateTouchDistance(touch1, touch2) {
    const dx = touch2.clientX - touch1.clientX;
    const dy = touch2.clientY - touch1.clientY;
    return Math.sqrt(dx * dx + dy * dy);
  }

  getTouchCenter(touch1, touch2) {
    return {
      x: (touch1.clientX + touch2.clientX) / 2,
      y: (touch1.clientY + touch2.clientY) / 2,
    };
  }

  updateLayerColor(layerId, hexColor) {
    const layer = this.layers.find((l) => l.id === layerId);
    if (!layer) return;

    const r = parseInt(hexColor.substr(1, 2), 16) / 255;
    const g = parseInt(hexColor.substr(3, 2), 16) / 255;
    const b = parseInt(hexColor.substr(5, 2), 16) / 255;

    layer.color = [r, g, b];
    this.render();
  }

  updateGlobalAlpha(alpha) {
    this.globalAlpha = alpha;
    // Only composite with new alpha (reuses FBO geometry)
    if (this.wasmProcessor) {
      this.wasmProcessor.composite(alpha);
    }
  }

  deleteLayer(layerId) {
    const index = this.layers.findIndex((l) => l.id === layerId);
    if (index !== -1) {
      const layer = this.layers[index];

      // remove from WASM processor
      this.wasmProcessor.remove_layer(layer.layerId);

      // remove from JS array
      this.layers.splice(index, 1);
    }

    this.renderLayerList();
    this.render();
  }

  clearAllLayers() {
    // remove all layers from WASM processor
    this.wasmProcessor.clear();

    this.layers = [];
    this.nextColorIndex = 0;
    this.renderLayerList();
    this.render();
  }

  selectAllLayerCheckboxes() {
    this.layers.forEach((layer) => {
      layer.visible = true;
    });
    this.renderLayerList();
    this.render();
  }

  unselectAllLayerCheckboxes() {
    this.layers.forEach((layer) => {
      layer.visible = false;
    });
    this.renderLayerList();
    this.render();
  }

  renderLayerList() {
    this.layerList.innerHTML = "";

    this.layers.forEach((layer) => {
      const li = document.createElement("li");
      li.className = "layer-item";
      li.dataset.layerId = layer.id;

      // Color picker
      const colorPicker = document.createElement("input");
      colorPicker.type = "color";
      colorPicker.className = "layer-color-picker";
      colorPicker.value = this.rgbToHex(layer.color);
      colorPicker.addEventListener("change", (e) => {
        this.updateLayerColor(layer.id, e.target.value);
      });

      // Checkbox
      const checkbox = document.createElement("input");
      checkbox.type = "checkbox";
      checkbox.className = "layer-checkbox";
      checkbox.checked = layer.visible;
      checkbox.addEventListener("change", () => {
        layer.visible = checkbox.checked;
        this.render();
      });

      // Label
      const label = document.createElement("label");
      label.className = "layer-label";
      label.textContent = layer.name;
      label.style.cursor = "pointer";
      label.addEventListener("click", () => {
        layer.visible = !layer.visible;
        checkbox.checked = layer.visible;
        this.render();
      });

      // Delete button
      const deleteBtn = document.createElement("button");
      deleteBtn.className = "layer-delete-btn";
      deleteBtn.innerHTML = `
        <svg width="10" height="10" viewBox="0 0 12 12" fill="none" xmlns="http://www.w3.org/2000/svg">
          <line x1="2" y1="6" x2="10" y2="6" stroke="white" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
      `;
      deleteBtn.setAttribute("aria-label", "Delete layer");
      deleteBtn.addEventListener("click", () => {
        this.deleteLayer(layer.id);
      });

      li.appendChild(colorPicker);
      li.appendChild(checkbox);
      li.appendChild(label);
      li.appendChild(deleteBtn);
      this.layerList.appendChild(li);
    });
  }

  rgbToHex(rgb) {
    const r = Math.round(rgb[0] * 255)
      .toString(16)
      .padStart(2, "0");
    const g = Math.round(rgb[1] * 255)
      .toString(16)
      .padStart(2, "0");
    const b = Math.round(rgb[2] * 255)
      .toString(16)
      .padStart(2, "0");
    return `#${r}${g}${b}`;
  }

  // Drawer management methods
  startDrawerResize(e) {
    e.preventDefault();
    this.isResizingDrawer = true;
    this.drawer.classList.add("resizing");
    document.body.style.userSelect = "none";
    document.body.style.cursor = "ew-resize";
  }

  resizeDrawer(e) {
    if (!this.isResizingDrawer) return;

    // Get X position from mouse or touch event
    const clientX = e.touches ? e.touches[0].clientX : e.clientX;

    // Calculate new width based on position
    // X position from right edge of viewport
    const newWidth = window.innerWidth - clientX;

    // Constrain width within min and max bounds
    if (newWidth >= this.drawerMinWidth && newWidth <= this.drawerMaxWidth) {
      this.drawerCurrentWidth = newWidth;
      this.drawer.style.width = `${newWidth}px`;
    }
  }

  stopDrawerResize(e) {
    if (!this.isResizingDrawer) return;

    this.isResizingDrawer = false;
    this.drawer.classList.remove("resizing");
    document.body.style.userSelect = "";
    document.body.style.cursor = "";

    // Trigger canvas resize
    this.triggerCanvasResize();
  }

  triggerCanvasResize() {
    // Dispatch resize event to notify canvas needs update
    window.dispatchEvent(new Event("resize"));
  }

  toggleDrawer() {
    const isCollapsed = this.drawer.classList.contains("collapsed");

    if (isCollapsed) {
      // Expand drawer
      this.drawer.classList.remove("collapsed");
      this.drawer.style.width = `${this.drawerCurrentWidth}px`;
      this.drawerToggleBtn.setAttribute("aria-label", "Hide drawer");
    } else {
      // Collapse drawer
      this.drawer.classList.add("collapsed");
      this.drawerToggleBtn.setAttribute("aria-label", "Show drawer");
    }

    // Trigger canvas resize immediately after toggle
    // Use requestAnimationFrame to ensure layout has updated
    requestAnimationFrame(() => {
      this.triggerCanvasResize();
    });
  }
}
