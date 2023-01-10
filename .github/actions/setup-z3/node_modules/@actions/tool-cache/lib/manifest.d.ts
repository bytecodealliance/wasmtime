export interface IToolReleaseFile {
    filename: string;
    platform: string;
    platform_version?: string;
    arch: string;
    download_url: string;
}
export interface IToolRelease {
    version: string;
    stable: boolean;
    release_url: string;
    files: IToolReleaseFile[];
}
export declare function _findMatch(versionSpec: string, stable: boolean, candidates: IToolRelease[], archFilter: string): Promise<IToolRelease | undefined>;
export declare function _getOsVersion(): string;
export declare function _readLinuxVersionFile(): string;
