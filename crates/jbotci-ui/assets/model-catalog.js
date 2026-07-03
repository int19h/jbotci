export function validateModelCatalog(raw, label = "embedding model catalog") {
  if (!raw || typeof raw !== "object" || Array.isArray(raw)) {
    throw new Error(`${label} must be an object`);
  }
  if (raw.schemaVersion !== 1) {
    throw new Error(`unsupported ${label} schema version: ${raw.schemaVersion}`);
  }
  const models = raw.models;
  if (!models || typeof models !== "object" || Array.isArray(models)) {
    throw new Error(`${label}.models must be an object`);
  }
  const normalizedModels = {};
  for (const [key, spec] of Object.entries(models)) {
    normalizedModels[key] = validateModelSpec(key, spec, label);
  }
  for (const field of ["defaultMobileModelKey", "defaultDesktopModelKey", "wasmFallbackModelKey"]) {
    if (typeof raw[field] !== "string" || !normalizedModels[raw[field]]) {
      throw new Error(`${label}.${field} must name a configured model`);
    }
  }
  const fallback = normalizedModels[raw.wasmFallbackModelKey];
  if (!fallback.wasmRuntime?.onnxUrl) {
    throw new Error(`${label}.wasmFallbackModelKey must provide wasmRuntime.onnxUrl`);
  }
  return {
    schemaVersion: 1,
    defaultMobileModelKey: raw.defaultMobileModelKey,
    defaultDesktopModelKey: raw.defaultDesktopModelKey,
    wasmFallbackModelKey: raw.wasmFallbackModelKey,
    models: normalizedModels,
  };
}

function validateModelSpec(key, spec, catalogLabel) {
  const label = `${catalogLabel} model ${key}`;
  if (!spec || typeof spec !== "object" || Array.isArray(spec)) {
    throw new Error(`${label} must be an object`);
  }
  const modelKey = requiredString(spec, "modelKey", label);
  if (modelKey !== key) {
    throw new Error(`${label} modelKey mismatch: ${modelKey}`);
  }
  const customRuntime = requiredObject(spec, "customRuntime", label);
  const preferredRuntime = requiredObject(spec, "preferredRuntime", label);
  const normalized = {
    modelKey,
    label: requiredString(spec, "label", label),
    modelId: requiredString(spec, "modelId", label),
    customRuntime: {
      runtime: requiredString(customRuntime, "runtime", `${label} customRuntime`),
      version: requiredString(customRuntime, "version", `${label} customRuntime`),
      artifactBaseUrl: requiredString(customRuntime, "artifactBaseUrl", `${label} customRuntime`),
      dtype: requiredString(customRuntime, "dtype", `${label} customRuntime`),
      device: requiredString(customRuntime, "device", `${label} customRuntime`),
    },
    preferredRuntime: {
      dtype: requiredString(preferredRuntime, "dtype", `${label} preferredRuntime`),
      device: requiredString(preferredRuntime, "device", `${label} preferredRuntime`),
    },
    dimensions: requiredPositiveInteger(spec, "dimensions", label),
    maxSequenceLength: requiredPositiveInteger(spec, "maxSequenceLength", label),
    queryPrefix: requiredString(spec, "queryPrefix", label),
    remoteVectorPacks: spec.remoteVectorPacks === true,
    browserLocalIndexing: spec.browserLocalIndexing !== false,
    localVectorSpaceKey: requiredString(spec, "localVectorSpaceKey", label),
    vectorElementType: requiredString(spec, "vectorElementType", label),
    embedBatchSize: requiredPositiveInteger(spec, "embedBatchSize", label),
    modelSizeEstimates: requiredObject(spec, "modelSizeEstimates", label),
    minFreeBytesByDtype: requiredObject(spec, "minFreeBytesByDtype", label),
    outputPooling: requiredString(spec, "outputPooling", label),
  };
  if (spec.wasmRuntime !== undefined && spec.wasmRuntime !== null) {
    const wasmRuntime = requiredObject(spec, "wasmRuntime", label);
    normalized.wasmRuntime = {
      runtime: requiredString(wasmRuntime, "runtime", `${label} wasmRuntime`),
      version: requiredString(wasmRuntime, "version", `${label} wasmRuntime`),
      onnxUrl: requiredString(wasmRuntime, "onnxUrl", `${label} wasmRuntime`),
      dtype: requiredString(wasmRuntime, "dtype", `${label} wasmRuntime`),
      device: requiredString(wasmRuntime, "device", `${label} wasmRuntime`),
    };
  }
  return normalized;
}

function requiredObject(value, field, label) {
  const fieldValue = value?.[field];
  if (!fieldValue || typeof fieldValue !== "object" || Array.isArray(fieldValue)) {
    throw new Error(`${label}.${field} must be an object`);
  }
  return fieldValue;
}

function requiredString(value, field, label) {
  const fieldValue = value?.[field];
  if (typeof fieldValue !== "string" || fieldValue.trim().length === 0) {
    throw new Error(`${label}.${field} must be a non-empty string`);
  }
  return fieldValue;
}

function requiredPositiveInteger(value, field, label) {
  const fieldValue = value?.[field];
  if (!Number.isInteger(fieldValue) || fieldValue <= 0) {
    throw new Error(`${label}.${field} must be a positive integer`);
  }
  return fieldValue;
}
