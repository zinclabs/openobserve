import http from "./http";

export const annotationService = {
  create_timed_annotations: (
    org_id: string,
    dashboard_id: string,
    annotations: Omit<any, "annotation_id">[],
  ) => {
    // Construct the request payload
    const data: any = {
      dashboard_id,
      timed_annotations: annotations.map((annotation) => ({
        annotation_id: crypto.randomUUID(),
        ...annotation,
      })),
    };

    // Make the API call
    return http().post(`/api/${org_id}/annotations`, data, {
      headers: {
        "Content-Type": "application/json; charset=UTF-8",
      },
    });
  },

  update_timed_annotations: (
    org_id: string,
    dashboard_id: string,
    annotations: any[],
  ) => {
    // Construct the request payload
    const data: any = {
      dashboard_id,
      timed_annotations: annotations.map((annotation) => ({
        ...annotation,
        // Ensure annotation_id is preserved and not regenerated
        annotation_id: annotation.annotation_id,
      })),
    };

    // Make the API call
    return http().put(`/api/${org_id}/annotations`, data, {
      headers: {
        "Content-Type": "application/json; charset=UTF-8",
      },
    });
  },

  delete_timed_annotations: (
    organization: string,
    dashboardId: string,
    annotationIds: string[],
  ) => {
    return http().delete(`/api/${organization}/annotations`, {
      data: {
        annotation_ids: annotationIds,
        dashboard_id: dashboardId,
      },
    });
  },

  get_timed_annotations: (
    org_id: string,
    dashboard_id: string,
    params: {
      panels: string[];
      start_time: number;
      end_time: number;
    },
  ) => {
    // Format the panels parameter as comma-separated string if it's an array
    const formattedPanels = Array.isArray(params.panels)
      ? params.panels.join(",")
      : params.panels;

    // Construct query parameters
    const queryParams = {
      dashboard_id,
      panels: formattedPanels,
      start_time: params.start_time,
      end_time: params.end_time,
    };

    // Make the API call
    return http().get(`/api/${org_id}/annotations`, { params: queryParams });
  },
};
