import {
  makeStyles,
  shorthands,
  FluentProvider,
  webLightTheme,
  Tab,
  TabList,
  Button,
  useToastController,
  Link, Toast, ToastBody, ToastFooter, ToastTitle,
  Toaster
} from "@fluentui/react-components";
import {
  PeopleAddRegular,
  PlayRegular,
  SettingsRegular,
  WalletCreditCardRegular,
} from "@fluentui/react-icons";
import { useId, useState } from "react";
import SettingsTab from "./tabs/Settings";
import AccountsTab from "./tabs/Accounts";
import TransactionsTab from "./tabs/Transactions";

const useStyles = makeStyles({
  root: {
    display: "flex",
    flexDirection: "column",
    justifyContent: "flex-start",
  },
  bar: {
    alignItems: "center",
    display: "flex",
    flexDirection: "row",
    justifyContent: "space-between",
    ...shorthands.padding("0px", "20px"),
    rowGap: "20px",
  },
  tabContent: {
    flexGrow: 1,
  }
});


const App = () => {
  const styles = useStyles();
  const [selectedTab, setSelectedTab] = useState("settings");

  const toasterId = useId();
  const { dispatchToast } = useToastController(toasterId);

  return (
    <FluentProvider theme={webLightTheme}>
      <main className={styles.root}>
        <header className={styles.bar}>
          <TabList onTabSelect={(_, data) => setSelectedTab(data.value as string)}>
            <Tab value="settings" icon={<SettingsRegular />}>Settings</Tab>
            <Tab value="accounts" icon={<PeopleAddRegular />}>Accounts</Tab>
            <Tab value="transactions" icon={<WalletCreditCardRegular />}>Transactions</Tab>
          </TabList>

          <Button icon={<PlayRegular />} appearance="primary" onClick={() => {
            dispatchToast(
              <Toast>
                <ToastTitle action={<Link>Undo</Link>}>Email sent</ToastTitle>
                <ToastBody subtitle="Subtitle">This is a toast body</ToastBody>
                <ToastFooter>
                  <Link>Action</Link>
                  <Link>Action</Link>
                </ToastFooter>
              </Toast>,
              {
                intent: "success",
              }
            )
          }}>Execute</Button>
        </header>
        <div className={styles.tabContent}>
          {selectedTab === "settings" && <SettingsTab />}
          {selectedTab === "accounts" && <AccountsTab />}
          {selectedTab === "transactions" && <TransactionsTab />}
        </div>
      </main>
      <Toaster toasterId={toasterId} />
    </FluentProvider >
  );
}

export default App;
