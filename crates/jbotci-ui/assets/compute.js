import { createWorkerClient } from "./worker-client.js";

const client = createWorkerClient({
  label: "compute",
  defaultWorkerUrl: () => new URL("./compute-worker.js", import.meta.url),
  minIdleWorkers: 1,
  maxIdleWorkers: 3,
  warmMessage: (context) => ({
    kind: "warm",
    mainModuleUrl: context.mainModuleUrl,
  }),
  requestMessage: ({ id, payload, workerEntry }) => ({
    id,
    requestJson: payload,
    mainModuleUrl: workerEntry.mainModuleUrl,
  }),
});

export function jbotciComputeConfigureWorker(workerUrlString) {
  client.configureWorker(workerUrlString);
}

export function jbotciComputeConfigureAppModule(appModuleUrlString) {
  client.configureAppModule(appModuleUrlString);
}

export function jbotciComputeCancel(channel) {
  client.cancel(channel);
}

export function jbotciComputeRequest(channel, requestJson) {
  return client.request(channel, requestJson);
}
