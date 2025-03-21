import { Card, Divider, Field, Input, makeStyles, Select } from "@fluentui/react-components";

type ValidateFunction = (value: string) => {
    validationState: "none" |
    "success" |
    "warning" |
    "error"
    validationMessage: string
}

const units = [
    "Wei",
    "Kwei",
    "Mwei",
    "Gwei",
    "Twei",
    "Pwei",
    "Ether"
]

const useStyles = makeStyles({
    form: {
        margin: "10px",
        minWidth: "420px",
        maxWidth: "100%",
        display: "flex",
        flexDirection: "row",
    },
    card: {
        padding: "20px",
        width: "100%",
    },
    flex: {
        display: "flex",
        flexDirection: "row",
    },
    inputWithUnit: {
        width: 'fill-available',
    }
});

const SettingsTab = () => {
    const styles = useStyles();
    return (
        <div className={styles.form}>
            <div className={styles.card}>
                <Field
                    label="Random Seed"
                >
                    <Input />
                </Field>
                <Field
                    label="Default Balance"
                >
                    <div className={styles.flex}>
                        <Input className={styles.inputWithUnit} />
                        <Select> {units.map(unit => <option> {unit} </option>)}</Select>
                    </div>
                </Field>
                <Field
                    label="Default Gas Price"
                >
                    <div className={styles.flex}>
                        <Input className={styles.inputWithUnit} />
                        <Select> {units.map(unit => <option> {unit} </option>)}</Select>
                    </div>
                </Field>
                <Field
                    label="Default Gas Limit"
                >
                    <div className={styles.flex}>
                        <Input className={styles.inputWithUnit} />
                        <Select> {units.map(unit => <option> {unit} </option>)}</Select>
                    </div>
                </Field>
            </div>
            <Divider vertical />
            <div className={styles.card}>
                <Field
                    label="Coinbase"
                >
                    <Input />
                </Field>
                <Field
                    label="Number"
                >
                    <Input />
                </Field>
                <Field
                    label="Timestamp"
                >
                    <Input />
                </Field>
                <Field
                    label="Gas Limit"
                >
                    <Input />
                </Field>
                <Field
                    label="Base Fee"
                >
                    <div className={styles.flex}>
                        <Input className={styles.inputWithUnit} />
                        <Select> {units.map(unit => <option> {unit} </option>)}</Select>
                    </div>
                </Field>
                <Field
                    label="Difficulty"
                >
                    <Input />
                </Field>
            </div>
        </div>
    );
}

export default SettingsTab;
