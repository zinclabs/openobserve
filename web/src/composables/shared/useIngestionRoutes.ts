// Copyright 2023 OpenObserve Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

import config from "@/aws-exports";
import { routeGuard } from "@/utils/zincutils";
import SysLog from "@/components/ingestion/logs/SysLog.vue";
import SyslogNg from "@/components/ingestion/logs/SyslogNg.vue";
import Ingestion from "@/views/Ingestion.vue";
import FluentBit from "@/components/ingestion/logs/FluentBit.vue";
import Fluentd from "@/components/ingestion/logs/Fluentd.vue";
import Vector from "@/components/ingestion/logs/Vector.vue";
import Curl from "@/components/ingestion/logs/Curl.vue";
import AWSConfig from "@/components/ingestion/recommended/AWSConfig.vue";
import GCPConfig from "@/components/ingestion/recommended/GCPConfig.vue";
import AzureConfig from "@/components/ingestion/recommended/AzureConfig.vue";
import FileBeat from "@/components/ingestion/logs/FileBeat.vue";
import OpenTelemetry from "@/components/ingestion/traces/OpenTelemetry.vue";
import PrometheusConfig from "@/components/ingestion/metrics/PrometheusConfig.vue";
import OtelCollector from "@/components/ingestion/metrics/OtelCollector.vue";
import TelegrafConfig from "@/components/ingestion/metrics/TelegrafConfig.vue";
import CloudWatchMetricConfig from "@/components/ingestion/metrics/CloudWatchMetrics.vue";
import IngestLogs from "@/components/ingestion/logs/Index.vue";
import IngestMetrics from "@/components/ingestion/metrics/Index.vue";
import IngestTraces from "@/components/ingestion/traces/Index.vue";
import Recommended from "@/components/ingestion/Recommended.vue";
import Custom from "@/components/ingestion/Custom.vue";
import LogstashDatasource from "@/components/ingestion/logs/LogstashDatasource.vue";

import RUMWeb from "@/components/ingestion/recommended/FrontendRumConfig.vue";
import KubernetesConfig from "@/components/ingestion/recommended/KubernetesConfig.vue";
import LinuxConfig from "@/components/ingestion/recommended/LinuxConfig.vue";
import OtelConfig from "@/components/ingestion/recommended/OtelConfig.vue";
import WindowsConfig from "@/components/ingestion/recommended/WindowsConfig.vue";

const useIngestionRoutes = () => {
  const ingestionRoutes: any = [
    {
      path: "ingestion",
      name: "ingestion",
      component: Ingestion,
      beforeEnter(to: any, from: any, next: any) {
        routeGuard(to, from, next);
      },
      children: [
        {
          path: "custom",
          name: "custom",
          component: Custom,
          beforeEnter(to: any, from: any, next: any) {
            routeGuard(to, from, next);
          },
          children: [
            {
              path: "logs",
              name: "ingestLogs",
              component: IngestLogs,
              beforeEnter(to: any, from: any, next: any) {
                routeGuard(to, from, next);
              },
              children: [
                {
                  path: "curl",
                  name: "curl",
                  component: Curl,
                  beforeEnter(to: any, from: any, next: any) {
                    routeGuard(to, from, next);
                  },
                },
                {
                  path: "fluentbit",
                  name: "fluentbit",
                  component: FluentBit,
                  beforeEnter(to: any, from: any, next: any) {
                    routeGuard(to, from, next);
                  },
                },
                {
                  path: "fluentd",
                  name: "fluentd",
                  component: Fluentd,
                  beforeEnter(to: any, from: any, next: any) {
                    routeGuard(to, from, next);
                  },
                },
                {
                  path: "vector",
                  name: "vector",
                  component: Vector,
                  beforeEnter(to: any, from: any, next: any) {
                    routeGuard(to, from, next);
                  },
                },
                {
                  path: "filebeat",
                  name: "filebeat",
                  component: FileBeat,
                  beforeEnter(to: any, from: any, next: any) {
                    routeGuard(to, from, next);
                  },
                },
                {
                  path: "otel",
                  name: "ingestLogsFromOtel",
                  component: OtelConfig,
                  beforeEnter(to: any, from: any, next: any) {
                    routeGuard(to, from, next);
                  },
                },
                {
                  path: "logstash",
                  name: "logstash",
                  component: LogstashDatasource,
                  beforeEnter(to: any, from: any, next: any) {
                    routeGuard(to, from, next);
                  },
                },
                {
                  path: "syslogng",
                  name: "syslogNg",
                  component: SyslogNg,
                  beforeEnter(to: any, from: any, next: any) {
                    routeGuard(to, from, next);
                  },
                },
              ],
            },
            {
              path: "metrics",
              name: "ingestMetrics",
              component: IngestMetrics,
              beforeEnter(to: any, from: any, next: any) {
                routeGuard(to, from, next);
              },
              children: [
                {
                  path: "prometheus",
                  name: "prometheus",
                  component: PrometheusConfig,
                  beforeEnter(to: any, from: any, next: any) {
                    routeGuard(to, from, next);
                  },
                },
                {
                  path: "otelcollector",
                  name: "otelCollector",
                  component: OtelCollector,
                  beforeEnter(to: any, from: any, next: any) {
                    routeGuard(to, from, next);
                  },
                },
                {
                  path: "telegraf",
                  name: "telegraf",
                  component: TelegrafConfig,
                  beforeEnter(to: any, from: any, next: any) {
                    routeGuard(to, from, next);
                  },
                },
                {
                  path: "cloudwatchMetrics",
                  name: "cloudwatchMetrics",
                  component: CloudWatchMetricConfig,
                  beforeEnter(to: any, from: any, next: any) {
                    routeGuard(to, from, next);
                  },
                },
              ],
            },
            {
              path: "traces",
              name: "ingestTraces",
              component: IngestTraces,
              beforeEnter(to: any, from: any, next: any) {
                routeGuard(to, from, next);
              },
              children: [
                {
                  path: "opentelemetry",
                  name: "tracesOTLP",
                  component: OpenTelemetry,
                  beforeEnter(to: any, from: any, next: any) {
                    routeGuard(to, from, next);
                  },
                },
                {
                  path: "otel",
                  name: "ingestTracesFromOtel",
                  component: OtelConfig,
                  beforeEnter(to: any, from: any, next: any) {
                    routeGuard(to, from, next);
                  },
                },
              ],
            },
          ],
        },
        {
          path: "recommended",
          name: "recommended",
          component: Recommended,
          beforeEnter(to: any, from: any, next: any) {
            routeGuard(to, from, next);
          },
          children: [
            {
              path: "kubernetes",
              name: "ingestFromKubernetes",
              component: KubernetesConfig,
              beforeEnter(to: any, from: any, next: any) {
                routeGuard(to, from, next);
              },
            },
            {
              path: "windows",
              name: "ingestFromWindows",
              component: WindowsConfig,
              beforeEnter(to: any, from: any, next: any) {
                routeGuard(to, from, next);
              },
            },
            {
              path: "linux",
              name: "ingestFromLinux",
              component: LinuxConfig,
              beforeEnter(to: any, from: any, next: any) {
                routeGuard(to, from, next);
              },
            },
            {
              path: "aws",
              name: "AWSConfig",
              component: AWSConfig,
              beforeEnter(to: any, from: any, next: any) {
                routeGuard(to, from, next);
              },
            },
            {
              path: "gcp",
              name: "GCPConfig",
              component: GCPConfig,
              beforeEnter(to: any, from: any, next: any) {
                routeGuard(to, from, next);
              },
            },
            {
              path: "azure",
              name: "AzureConfig",
              component: AzureConfig,
              beforeEnter(to: any, from: any, next: any) {
                routeGuard(to, from, next);
              },
            },
            {
              path: "traces",
              name: "ingestFromTraces",
              component: OpenTelemetry,
              beforeEnter(to: any, from: any, next: any) {
                routeGuard(to, from, next);
              },
            },
            {
              path: "frontend-monitoring",
              name: "frontendMonitoring",
              component: RUMWeb,
              beforeEnter(to: any, from: any, next: any) {
                routeGuard(to, from, next);
              },
            },
          ],
        },
      ],
    },
  ];

  const sysLog = {
    path: "syslog",
    name: "syslog",
    component: SysLog,
    beforeEnter(to: any, from: any, next: any) {
      routeGuard(to, from, next);
    },
  };

  // const sysLogNg = {
  //   path: "syslogng",
  //   name: "syslogNg",
  //   component: SyslogNg,
  //   beforeEnter(to: any, from: any, next: any) {
  //     routeGuard(to, from, next);
  //   },
  // };

  if (config.isCloud === "false" || !config.isCloud) {
    ingestionRoutes[0].children
      .find((child: any) => child.name === "custom")
      .children.find((child: any) => child.name === "ingestLogs")
      ?.children.push(...[sysLog]);
  }

  return ingestionRoutes;
};

export default useIngestionRoutes;
