import init, { SchematicConverter, SchematicFormat, init_panic_hook } from './schematic_converter.js';
async function run() {
    await init();
    init_panic_hook();
    const converter = new SchematicConverter();

    const fileInput = document.getElementById('fileInput');
    const fromFormat = document.getElementById('fromFormat');
    const toFormat = document.getElementById('toFormat');
    const convertButton = document.getElementById('convertButton');
    const output = document.getElementById('output');

    convertButton.addEventListener('click', async () => {
        const file = fileInput.files[0];
        if (!file) {
            output.textContent = 'Please select a file.';
            return;
        }

        const buffer = await file.arrayBuffer();
        const inputArray = new Uint8Array(buffer);

        try {
            const result = converter.convert(
                inputArray,
                SchematicFormat[fromFormat.value],
                SchematicFormat[toFormat.value]
            );

            const blob = new Blob([result], { type: 'application/octet-stream' });
            const url = URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = `converted.${toFormat.value.toLowerCase()}`;
            a.click();
            URL.revokeObjectURL(url);

            output.textContent = 'Conversion successful! File downloaded.';
        } catch (error) {
            output.textContent = `Conversion failed: ${error}`;
        }
    });
}

run().catch(console.error);