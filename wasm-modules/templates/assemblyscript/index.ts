import "wasi"
import { Console, FileSystem, Descriptor } from "as-wasi";

Console.log('Hello World!\n')

let filePath: string = "/work/result.json";
let fileOrNull: Descriptor | null = FileSystem.open(filePath, "w+");

if (fileOrNull == null) {
    throw new Error("Could not open the file " + filePath);
}

let file = changetype<Descriptor>(fileOrNull);

file.writeStringLn("{\"phase\":\"Succeeded\",\"message\":\"Hello\",\"outputs\":{\"artifacts\":[],\"parameters\":[]}}");
