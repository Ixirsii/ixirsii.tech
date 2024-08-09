import Header from "./_components/header";
import Keyboards from "./_components/keyboards";

export default async function Home() {
  return (
    <>
      <Header />

      <main className="flex min-h-screen flex-col items-center justify-center bg-gray-200">
        <Keyboards />
      </main>
    </>
  );
}
