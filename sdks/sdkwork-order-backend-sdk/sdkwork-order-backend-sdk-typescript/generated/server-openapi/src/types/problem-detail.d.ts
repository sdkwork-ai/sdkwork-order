import type { FieldError } from './field-error';
import type { SdkWorkPlatformErrorCode } from './sdk-work-platform-error-code';
export interface ProblemDetail {
    type: string;
    title: string;
    status: number;
    detail?: string;
    instance?: string;
    code: SdkWorkPlatformErrorCode;
    traceId: string;
    errors?: FieldError[];
}
//# sourceMappingURL=problem-detail.d.ts.map