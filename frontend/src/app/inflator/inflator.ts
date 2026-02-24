import { WebInflator } from "@denshya/tama"

import applyCustomAttributes from "./custom-attributes"

const inflator = new WebInflator
inflator.flags.debug = import.meta.env.DEV

applyCustomAttributes(inflator)

export default inflator
