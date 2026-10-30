/** exifr's `lite` bundle (JPEG and HEIC) has the same API as the package's main entry. */
declare module "exifr/dist/lite.esm.mjs" {
  import exifr from "exifr";
  export default exifr;
}
