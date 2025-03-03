`timescale 1ns / 1ps

import test_cases::*;

module segments_tb;
    test_case current_test;

    digit bs_10, bs_1, ws_10, ws_1, m_10, m_1, s_10, s_1;
    t_o_time bto, wto;
    logic bto_ind, wto_ind, rto_ind;
    logic fst_hlf, hlf_tm, snd_hlf, overtime, sdn_dth;
    logic colon;
    logic [1:0] brightness;

    logic test_failed = 0;

    segments segments(
        .data(current_test.data),
        .bs_10(bs_10),
        .bs_1(bs_1),
        .ws_10(ws_10),
        .ws_1(ws_1),
        .m_10(m_10),
        .m_1(m_1),
        .s_10(s_10),
        .s_1(s_1),
        .bto(bto),
        .wto(wto),
        .bto_ind(bto_ind),
        .wto_ind(wto_ind),
        .rto_ind(rto_ind),
        .fst_hlf(fst_hlf),
        .hlf_tm(hlf_tm),
        .snd_hlf(snd_hlf),
        .overtime(overtime),
        .sdn_dth(sdn_dth),
        .colon(colon),
        .brightness(brightness)
    );

    initial begin
        foreach (all_tests[i]) begin
            current_test = all_tests[i];
            test_failed = 0;

            #10; // Wait for 10 time units

            if (bs_10 !== current_test.bs_10_ex) begin
                $error("Test \"%s\" failed: bs_10 is %b, expected %b", current_test.name, bs_10, current_test.bs_10_ex);
                test_failed = 1;
            end
            if (bs_1 !== current_test.bs_1_ex) begin
                $error("Test \"%s\" failed: bs_1 is %b, expected %b", current_test.name, bs_1, current_test.bs_1_ex);
                test_failed = 1;
            end
            if (ws_10 !== current_test.ws_10_ex) begin
                $error("Test \"%s\" failed: ws_10 is %b, expected %b", current_test.name, ws_10, current_test.ws_10_ex);
                test_failed = 1;
            end
            if (ws_1 !== current_test.ws_1_ex) begin
                $error("Test \"%s\" failed: ws_1 is %b, expected %b", current_test.name, ws_1, current_test.ws_1_ex);
                test_failed = 1;
            end
            if (m_10 !== current_test.m_10_ex) begin
                $error("Test \"%s\" failed: m_10 is %b, expected %b", current_test.name, m_10, current_test.m_10_ex);
                test_failed = 1;
            end
            if (m_1 !== current_test.m_1_ex) begin
                $error("Test \"%s\" failed: m_1 is %b, expected %b", current_test.name, m_1, current_test.m_1_ex);
                test_failed = 1;
            end
            if (s_10 !== current_test.s_10_ex) begin
                $error("Test \"%s\" failed: s_10 is %b, expected %b", current_test.name, s_10, current_test.s_10_ex);
                test_failed = 1;
            end
            if (s_1 !== current_test.s_1_ex) begin
                $error("Test \"%s\" failed: s_1 is %b, expected %b", current_test.name, s_1, current_test.s_1_ex);
                test_failed = 1;
            end
            if (bto !== current_test.bto_ex) begin
                $error("Test \"%s\" failed: bto is %b, expected %b", current_test.name, bto, current_test.bto_ex);
                test_failed = 1;
            end
            if (wto !== current_test.wto_ex) begin
                $error("Test \"%s\" failed: wto is %b, expected %b", current_test.name, wto, current_test.wto_ex);
                test_failed = 1;
            end
            if (bto_ind !== current_test.bto_ind_ex) begin
                $error("Test \"%s\" failed: bto_ind is %b, expected %b", current_test.name, bto_ind, current_test.bto_ind_ex);
                test_failed = 1;
            end
            if (wto_ind !== current_test.wto_ind_ex) begin
                $error("Test \"%s\" failed: wto_ind is %b, expected %b", current_test.name, wto_ind, current_test.wto_ind_ex);
                test_failed = 1;
            end
            if (rto_ind !== current_test.rto_ind_ex) begin
                $error("Test \"%s\" failed: rto_ind is %b, expected %b", current_test.name, rto_ind, current_test.rto_ind_ex);
                test_failed = 1;
            end
            if (fst_hlf !== current_test.fst_hlf_ex) begin
                $error("Test \"%s\" failed: fst_hlf is %b, expected %b", current_test.name, fst_hlf, current_test.fst_hlf_ex);
                test_failed = 1;
            end
            if (hlf_tm !== current_test.hlf_tm_ex) begin
                $error("Test \"%s\" failed: hlf_tm is %b, expected %b", current_test.name, hlf_tm, current_test.hlf_tm_ex);
                test_failed = 1;
            end
            if (snd_hlf !== current_test.snd_hlf_ex) begin
                $error("Test \"%s\" failed: snd_hlf is %b, expected %b", current_test.name, snd_hlf, current_test.snd_hlf_ex);
                test_failed = 1;
            end
            if (overtime !== current_test.overtime_ex) begin
                $error("Test \"%s\" failed: overtime is %b, expected %b", current_test.name, overtime, current_test.overtime_ex);
                test_failed = 1;
            end
            if (sdn_dth !== current_test.sdn_dth_ex) begin
                $error("Test \"%s\" failed: sdn_dth is %b, expected %b", current_test.name, sdn_dth, current_test.sdn_dth_ex);
                test_failed = 1;
            end
            if (colon !== current_test.colon_ex) begin
                $error("Test \"%s\" failed: colon is %b, expected %b", current_test.name, colon, current_test.colon_ex);
                test_failed = 1;
            end
            if (brightness !== current_test.brightness_ex) begin
                $error("Test \"%s\" failed: brightness is %b, expected %b", current_test.name, brightness, current_test.brightness_ex);
                test_failed = 1;
            end

            if (!test_failed) begin
                $display("Test \"%s\" Succeeded", current_test.name);
            end
        end
    end

endmodule